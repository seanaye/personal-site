//! Embeddable dumbpipe-compatible TCP connector.
//!
//! This crate implements the `dumbpipe connect-tcp` side as a library. It binds
//! a local TCP listener and forwards every accepted connection over Iroh to a
//! remote `dumbpipe listen-tcp` process.
//!
//! The wire protocol intentionally matches dumbpipe:
//! - ALPN: [`ALPN`]
//! - the connecting side opens a bidirectional stream
//! - the connecting side writes [`HANDSHAKE`] before forwarding bytes
//!
//! ```no_run
//! use dumbpipe_client::{DumbpipeTunnel, EndpointTicket, TunnelConfig};
//!
//! # async fn run() -> Result<(), dumbpipe_client::Error> {
//! let ticket: EndpointTicket = std::env::var("IMMICH_IROH_TICKET")?.parse()?;
//! let tunnel = DumbpipeTunnel::connect(ticket, TunnelConfig::default()).await?;
//! let immich_base_url = format!("http://{}", tunnel.local_addr());
//! # let _ = immich_base_url;
//! # Ok(())
//! # }
//! ```

use std::{
    future::Future,
    io,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    time::Duration,
};

use iroh::{
    endpoint::{presets, Connection},
    Endpoint, EndpointAddr, EndpointId, SecretKey,
};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::{TcpListener, TcpStream},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

/// The ALPN used by dumbpipe.
pub const ALPN: &[u8] = b"DUMBPIPEV0";

/// The fixed handshake sent by the side that opens the bidirectional stream.
pub const HANDSHAKE: [u8; 5] = *b"hello";

const DEFAULT_ONLINE_TIMEOUT: Duration = Duration::from_secs(5);

pub use iroh_tickets::endpoint::EndpointTicket;

/// Configuration for the local connector tunnel.
#[derive(Debug, Clone)]
pub struct TunnelConfig {
    /// Local TCP address to bind for callers inside this process/host.
    ///
    /// The default is `127.0.0.1:0`, which asks the OS for an unused loopback
    /// port. Prefer loopback addresses; binding to `0.0.0.0` exposes the
    /// remote service to the public network.
    pub bind_addr: SocketAddr,

    /// Optional stable Iroh secret key for this local endpoint.
    ///
    /// The connector side does not normally need a stable identity, so the
    /// default is to generate an ephemeral key.
    pub secret_key: Option<SecretKey>,

    /// Optional IPv4 bind address for the Iroh endpoint itself.
    pub endpoint_ipv4_addr: Option<SocketAddrV4>,

    /// Optional IPv6 bind address for the Iroh endpoint itself.
    pub endpoint_ipv6_addr: Option<SocketAddrV6>,

    /// ALPN to use when connecting to the remote endpoint.
    ///
    /// Leave as [`ALPN`] when connecting to normal dumbpipe.
    pub alpn: Vec<u8>,

    /// Whether to send the dumbpipe handshake after opening each stream.
    ///
    /// Leave this enabled when connecting to normal dumbpipe. Disable only when
    /// intentionally using a custom ALPN/protocol.
    pub send_handshake: bool,

    /// How long to wait for the local Iroh endpoint to come online before
    /// starting the TCP listener anyway.
    pub online_timeout: Duration,
}

impl Default for TunnelConfig {
    fn default() -> Self {
        Self {
            bind_addr: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0)),
            secret_key: None,
            endpoint_ipv4_addr: None,
            endpoint_ipv6_addr: None,
            alpn: ALPN.to_vec(),
            send_handshake: true,
            online_timeout: DEFAULT_ONLINE_TIMEOUT,
        }
    }
}

impl TunnelConfig {
    /// Bind a tunnel to a specific local TCP address.
    pub fn with_bind_addr(mut self, bind_addr: SocketAddr) -> Self {
        self.bind_addr = bind_addr;
        self
    }

    /// Use a specific Iroh endpoint secret key.
    pub fn with_secret_key(mut self, secret_key: SecretKey) -> Self {
        self.secret_key = Some(secret_key);
        self
    }

    /// Use a custom ALPN and disable the dumbpipe handshake.
    pub fn with_custom_alpn(mut self, alpn: impl Into<Vec<u8>>) -> Self {
        self.alpn = alpn.into();
        self.send_handshake = false;
        self
    }
}

/// A running local TCP listener forwarding to a remote dumbpipe endpoint.
pub struct DumbpipeTunnel {
    local_addr: SocketAddr,
    shutdown: CancellationToken,
    task: Option<JoinHandle<Result<(), Error>>>,
}

impl DumbpipeTunnel {
    /// Start a local connector tunnel from an endpoint ticket.
    pub async fn connect(ticket: EndpointTicket, config: TunnelConfig) -> Result<Self, Error> {
        let remote_addr = ticket.endpoint_addr().clone();
        let endpoint = create_endpoint(&config).await?;

        if tokio::time::timeout(config.online_timeout, endpoint.online())
            .await
            .is_err()
        {
            tracing::warn!("Iroh endpoint did not report online before timeout");
        }

        let listener = TcpListener::bind(config.bind_addr).await?;
        let local_addr = listener.local_addr()?;
        let shutdown = CancellationToken::new();
        let task = tokio::spawn(accept_loop(
            listener,
            endpoint,
            remote_addr,
            config.alpn,
            config.send_handshake,
            shutdown.clone(),
        ));

        Ok(Self {
            local_addr,
            shutdown,
            task: Some(task),
        })
    }

    /// Address the embedded application should use as its Immich base URL host.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Stop accepting new local connections and wait for the listener task to exit.
    pub async fn shutdown(mut self) -> Result<(), Error> {
        self.shutdown.cancel();
        if let Some(task) = self.task.take() {
            task.await??;
        }
        Ok(())
    }
}

impl Drop for DumbpipeTunnel {
    fn drop(&mut self) {
        self.shutdown.cancel();
    }
}

async fn create_endpoint(config: &TunnelConfig) -> Result<Endpoint, Error> {
    let secret_key = config
        .secret_key
        .clone()
        .unwrap_or_else(SecretKey::generate);
    let mut builder = Endpoint::builder(presets::N0)
        .secret_key(secret_key)
        .alpns(Vec::<Vec<u8>>::new());

    if let Some(addr) = config.endpoint_ipv4_addr {
        builder = builder.bind_addr(addr).map_err(Error::iroh)?;
    }

    if let Some(addr) = config.endpoint_ipv6_addr {
        builder = builder.bind_addr(addr).map_err(Error::iroh)?;
    }

    builder.bind().await.map_err(Error::iroh)
}

async fn accept_loop(
    listener: TcpListener,
    endpoint: Endpoint,
    remote_addr: EndpointAddr,
    alpn: Vec<u8>,
    send_handshake: bool,
    shutdown: CancellationToken,
) -> Result<(), Error> {
    loop {
        let (tcp_stream, tcp_addr) = tokio::select! {
            result = listener.accept() => result?,
            _ = shutdown.cancelled() => break,
        };

        tracing::debug!(%tcp_addr, remote = %remote_addr.id, "accepted local TCP connection");

        tokio::spawn(handle_tcp_connection(
            tcp_stream,
            endpoint.clone(),
            remote_addr.clone(),
            alpn.clone(),
            send_handshake,
        ));
    }

    Ok(())
}

async fn handle_tcp_connection(
    tcp_stream: TcpStream,
    endpoint: Endpoint,
    remote_addr: EndpointAddr,
    alpn: Vec<u8>,
    send_handshake: bool,
) {
    if let Err(error) = forward_tcp_connection(
        tcp_stream,
        endpoint,
        remote_addr.clone(),
        &alpn,
        send_handshake,
    )
    .await
    {
        tracing::warn!(remote = %remote_addr.id, %error, "failed to forward TCP connection");
    }
}

async fn forward_tcp_connection(
    tcp_stream: TcpStream,
    endpoint: Endpoint,
    remote_addr: EndpointAddr,
    alpn: &[u8],
    send_handshake: bool,
) -> Result<(), Error> {
    let remote_id = remote_addr.id;
    let connection = endpoint
        .connect(remote_addr, alpn)
        .await
        .map_err(Error::iroh)?;
    let (endpoint_send, endpoint_recv) = open_dumbpipe_stream(connection, send_handshake).await?;
    let (tcp_recv, tcp_send) = tcp_stream.into_split();

    forward_bidi(tcp_recv, tcp_send, endpoint_recv, endpoint_send)
        .await
        .map_err(|source| Error::Forward { remote_id, source })
}

async fn open_dumbpipe_stream(
    connection: Connection,
    send_handshake: bool,
) -> Result<(noq::SendStream, noq::RecvStream), Error> {
    let (mut send, recv) = connection.open_bi().await.map_err(Error::iroh)?;

    if send_handshake {
        send.write_all(&HANDSHAKE).await.map_err(Error::iroh)?;
    }

    Ok((send, recv))
}

async fn forward_bidi(
    from_tcp: impl AsyncRead + Send + Sync + Unpin + 'static,
    to_tcp: impl AsyncWrite + Send + Sync + Unpin + 'static,
    from_remote: noq::RecvStream,
    to_remote: noq::SendStream,
) -> io::Result<()> {
    let token = CancellationToken::new();
    let token_to_remote = token.clone();
    let token_from_remote = token.clone();

    let to_remote = tokio::spawn(async move {
        copy_to_noq(from_tcp, to_remote, token_to_remote.clone())
            .await
            .map_err(cancel_token(token_to_remote))
    });
    let from_remote = tokio::spawn(async move {
        copy_from_noq(from_remote, to_tcp, token_from_remote.clone())
            .await
            .map_err(cancel_token(token_from_remote))
    });

    from_remote.await.map_err(join_error_to_io)??;
    to_remote.await.map_err(join_error_to_io)??;
    Ok(())
}

async fn copy_to_noq(
    mut from: impl AsyncRead + Unpin,
    mut send: noq::SendStream,
    token: CancellationToken,
) -> io::Result<u64> {
    tokio::select! {
        result = tokio::io::copy(&mut from, &mut send) => {
            let copied = result?;
            send.finish()?;
            Ok(copied)
        }
        _ = token.cancelled() => {
            send.reset(0u8.into()).ok();
            Err(io::Error::other("cancelled"))
        }
    }
}

async fn copy_from_noq(
    mut recv: noq::RecvStream,
    mut to: impl AsyncWrite + Unpin,
    token: CancellationToken,
) -> io::Result<u64> {
    tokio::select! {
        result = tokio::io::copy(&mut recv, &mut to) => result,
        _ = token.cancelled() => {
            recv.stop(0u8.into()).ok();
            Err(io::Error::other("cancelled"))
        }
    }
}

fn cancel_token<T>(token: CancellationToken) -> impl FnOnce(T) -> T {
    move |value| {
        token.cancel();
        value
    }
}

fn join_error_to_io(error: tokio::task::JoinError) -> io::Error {
    io::Error::other(error)
}

/// Error returned by tunnel setup and shutdown.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    TicketParse(#[from] iroh_tickets::ParseError),

    #[error("Iroh error: {0}")]
    Iroh(String),

    #[error("forwarding connection to `{remote_id}` failed")]
    Forward {
        remote_id: EndpointId,
        #[source]
        source: io::Error,
    },

    #[error(transparent)]
    Join(#[from] tokio::task::JoinError),
}

impl Error {
    fn iroh(error: impl std::fmt::Display) -> Self {
        Self::Iroh(error.to_string())
    }
}

impl From<std::env::VarError> for Error {
    fn from(error: std::env::VarError) -> Self {
        Self::Io(io::Error::new(io::ErrorKind::NotFound, error))
    }
}

#[allow(dead_code)]
fn _assert_send_sync<T: Send + Sync>() {}

#[allow(dead_code)]
fn _assert_future_send<F: Future + Send>(future: F) -> F {
    future
}
