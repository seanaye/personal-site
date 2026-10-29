use liquid::{
    color::{hue_from_cookie, palette_css_properties, palette_for_slider},
    palette::PolineManagerImpl,
};
use topcoat::{
    context::Cx,
    router::{header::COOKIE, request::headers},
};

pub(crate) fn initial_hue(cx: &Cx) -> f64 {
    headers(cx)
        .get_all(COOKIE)
        .iter()
        .filter_map(|header| header.to_str().ok())
        .find_map(hue_from_cookie)
        .unwrap_or(0.0)
}

pub(crate) fn initial_palette(cx: &Cx) -> PolineManagerImpl {
    palette_for_slider(initial_hue(cx))
}

pub(crate) fn document_palette_style(cx: &Cx) -> String {
    palette_css_properties(&initial_palette(cx))
        .into_iter()
        .map(|(name, value)| format!("{name}: {value};"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{debug_poline, liquid_canvas, nav_bar, static_canvas};
    use liquid::{
        color::{canvas_background_style, rgb_css},
        palette::PolineManager,
    };
    use topcoat::{
        Result,
        router::{Body, Router, StatusCode, page, to_bytes},
        view::{View, view},
    };

    // Exercise the actual SSR components without loading assets or a browser.
    #[page("/")]
    async fn palette_page(cx: &Cx) -> Result<impl View> {
        Ok(view! {
            <html style=(document_palette_style(cx))>
                <body>
                    liquid_canvas()
                    static_canvas()
                    nav_bar()
                    debug_poline()
                </body>
            </html>
        })
    }

    #[tokio::test]
    async fn server_renders_request_scoped_hue_before_browser_initialization() {
        let router = Router::builder().page(palette_page).build();
        for (cookie, hue) in [
            (Some("other=1; liquid-hue-v2=171"), 171.0),
            (Some("liquid-hue-v2=360"), 360.0),
            (None, 0.0),
            (Some("liquid-hue-v2=NaN"), 0.0),
            (Some("liquid-hue-v2=-1"), 0.0),
            (Some("liquid-hue-v2=361"), 0.0),
            (Some("liquid-hue-v2=0"), 0.0),
        ] {
            let mut request = http::Request::builder().uri("/");
            if let Some(cookie) = cookie {
                request = request.header(COOKIE, cookie);
            }
            let response = router.handle(request.body(Body::empty()).unwrap()).await;
            assert_eq!(response.status(), StatusCode::OK);
            let body = to_bytes(response.into_body(), 1_000_000).await.unwrap();
            let html = std::str::from_utf8(&body).unwrap();
            let palette = palette_for_slider(hue);

            assert!(html.contains(&format!("value=\"{hue}\"")));
            assert!(html.contains(&canvas_background_style(&palette)));
            assert!(html.contains("<liquid-canvas"));
            assert!(html.contains("background: var(--poline-background-color);"));
            for (name, value) in palette_css_properties(&palette) {
                assert!(html.contains(&format!("{name}: {value};")));
            }
            assert!(html.contains(&format!(
                "background-color: {};",
                rgb_css(palette.colors()[0]),
            )));
        }
    }
}
