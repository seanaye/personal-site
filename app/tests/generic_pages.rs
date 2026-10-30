//! Guard the generic-page support supplied by the pinned Topcoat fork.
use topcoat::{
    Result,
    context::{Cx, context},
    router::{Body, Router, StatusCode, href, page, request::Request, to_bytes},
    view::{View, view},
};

trait PageState: Send + Sync + 'static {
    fn title(&self) -> &str;
}

// T is used only in the body, and the page still returns an unboxed impl View.
#[page("/generic")]
async fn generic_page<T: PageState>(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <h1>(context::<T>(cx).title())</h1>
        <a href=(href!(generic_page::<T>::default()))>"Self"</a>
    })
}

fn router<T: PageState>(state: T) -> Router {
    Router::builder()
        .with(state)
        .page(generic_page::<T>::default())
        .build()
}

// Neither implementation derives Default: constructing the marker must not
// require constructing the state, which comes from the composition layer.
struct ProductionState(String);
impl PageState for ProductionState {
    fn title(&self) -> &str {
        &self.0
    }
}

struct TestState;
impl PageState for TestState {
    fn title(&self) -> &str {
        "Test"
    }
}

#[tokio::test]
async fn generic_pages_use_the_state_selected_at_registration() {
    for (router, title) in [
        (
            router(ProductionState("Production".to_owned())),
            "Production",
        ),
        (router(TestState), "Test"),
    ] {
        let request = Request::builder()
            .uri("/generic")
            .body(Body::empty())
            .unwrap();
        let response = router.handle(request).await;
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), 4096).await.unwrap();
        assert_eq!(
            std::str::from_utf8(&bytes).unwrap(),
            format!("<h1>{title}</h1><a href=\"/generic\">Self</a>")
        );
    }
}
