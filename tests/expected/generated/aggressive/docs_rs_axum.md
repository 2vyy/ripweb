# Crate axum &nbsp; Copy item path
[Source](../src/axum/lib.rs.html#1-488) Expand description
axum is a web application framework that focuses on ergonomics and modularity.

## [§](#high-level-features) High-level features

- Route requests to handlers with a macro-free API.
- Declaratively parse requests using extractors.
- Simple and predictable error handling model.
- Generate responses with minimal boilerplate.
- Take full advantage of the [`tower`](https://crates.io/crates/tower) and [`tower-http`](https://crates.io/crates/tower-http) ecosystem of middleware, services, and utilities.

In particular, the last point is what sets `axum` apart from other frameworks. `axum` doesn’t have its own middleware system but instead uses [`tower::Service`](https://docs.rs/tower-service/0.3.3/x86_64-unknown-linux-gnu/tower_service/trait.Service.html). This means `axum` gets timeouts, tracing, compression, authorization, and more, for free. It also enables you to share middleware with applications written using [`hyper`](http://crates.io/crates/hyper) or [`tonic`](http://crates.io/crates/tonic).

## [§](#compatibility) Compatibility

axum is designed to work with [tokio](https://docs.rs/tokio/1.47.1/x86_64-unknown-linux-gnu/tokio/index.html) and [hyper](https://docs.rs/hyper/1.7.0/x86_64-unknown-linux-gnu/hyper/index.html). Runtime and transport layer independence is not a goal, at least for the time being.

## [§](#example) Example

The “Hello, World!” of axum is:

```
use axum::{
routing::get,
Router,
};
#[tokio::main]
async fn main() {
// build our application with a single route
let app = Router::new().route("/", get(|| async {"Hello, World!"}));
// run our app with hyper, listening globally on port 3000
let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
axum::serve(listener, app).await.unwrap();
}
```

Note using `#[tokio::main]` requires you enable tokio’s `macros` and `rt-multi-thread` features or just `full` to enable all features (`cargo add tokio --features macros,rt-multi-thread`).

## [§](#routing) Routing

[`Router`](struct.Router.html) is used to set up which paths go to which services:

```
use axum::{Router, routing::get};
// our router
let app = Router::new()
.route("/", get(root))
.route("/foo", get(get_foo).post(post_foo))
.route("/foo/bar", get(foo_bar));
// which calls one of these handlers
async fn root() {}
async fn get_foo() {}
async fn post_foo() {}
async fn foo_bar() {}
```

See [`Router`](struct.Router.html) for more details on routing.

## [§](#handlers) Handlers

In axum a “handler” is an async function that accepts zero or more [“extractors”](extract/index.html) as arguments and returns something that can be converted [into a response](response/index.html).

Handlers are where your application logic lives and axum applications are built by routing between handlers.

See [`handler`](handler/index.html) for more details on handlers.

## [§](#extractors) Extractors

An extractor is a type that implements [`FromRequest`](extract/trait.FromRequest.html) or [`FromRequestParts`](extract/trait.FromRequestParts.html). Extractors are how you pick apart the incoming request to get the parts your handler needs.

```
use axum::extract::{Path, Query, Json};
use std::collections::HashMap;
// `Path` gives you the path parameters and deserializes them.
async fn path(Path(user_id): Path&lt;u32&gt;) {}
// `Query` gives you the query parameters and deserializes them.
async fn query(Query(params): Query&lt;HashMap&lt;String, String&gt;&gt;) {}
// Buffer the request body and deserialize it as JSON into a
// `serde_json::Value`. `Json` supports any type that implements
// `serde::Deserialize`.
async fn json(Json(payload): Json&lt;serde_json::Value&gt;) {}
```

See [`extract`](extract/index.html) for more details on extractors.

## [§](#responses) Responses

Anything that implements [`IntoResponse`](response/trait.IntoResponse.html) can be returned from handlers.

```
use axum::{
body::Body,
routing::get,
response::Json,
Router,
};
use serde_json::{Value, json};
// `&amp;'static str` becomes a `200 OK` with `content-type: text/plain; charset=utf-8`
async fn plain_text() -&gt; &amp;'static str {
"foo"
}
// `Json` gives a content-type of `application/json` and works with any type
// that implements `serde::Serialize`
async fn json() -&gt; Json&lt;Value&gt; {
Json(json!({"data": 42}))
}
let app = Router::new()
.route("/plain_text", get(plain_text))
.route("/json", get(json));
```

See [`response`](response/index.html) for more details on building responses.

## [§](#error-handling) Error handling

axum aims to have a simple and predictable error handling model. That means it is simple to convert errors into responses and you are guaranteed that all errors are handled.

See [`error_handling`](error_handling/index.html) for more details on axum’s error handling model and how to handle errors gracefully.

## [§](#middleware) Middleware

There are several different ways to write middleware for axum. See [`middleware`](middleware/index.html) for more details.

## [§](#sharing-state-with-handlers) Sharing state with handlers

It is common to share some state between handlers. For example, a pool of database connections or clients to other services may need to be shared.

The four most common ways of doing that are:

- Using the [`State`](extract/struct.State.html) extractor
- Using request extensions
- Using closure captures
- Using task-local variables

### [§](#using-the-state-extractor) Using the [`State`](extract/struct.State.html) extractor

```
use axum::{
extract::State,
routing::get,
Router,
};
use std::sync::Arc;
struct AppState {
//...
}
let shared_state = Arc::new(AppState {/*... */});
let app = Router::new()
.route("/", get(handler))
.with_state(shared_state);
async fn handler(
State(state): State&lt;Arc&lt;AppState&gt;&gt;,
) {
//...
}
```

You should prefer using [`State`](extract/struct.State.html) if possible since it’s more type safe. The downside is that it’s less dynamic than task-local variables and request extensions.

See [`State`](extract/struct.State.html) for more details about accessing state.

### [§](#using-request-extensions) Using request extensions

Another way to share state with handlers is using [`Extension`](struct.Extension.html) as layer and extractor:

```
use axum::{
extract::Extension,
routing::get,
Router,
};
use std::sync::Arc;
struct AppState {
//...
}
let shared_state = Arc::new(AppState {/*... */});
let app = Router::new()
.route("/", get(handler))
.layer(Extension(shared_state));
async fn handler(
Extension(state): Extension&lt;Arc&lt;AppState&gt;&gt;,
) {
//...
}
```

The downside to this approach is that you’ll get runtime errors (specifically a `500 Internal Server Error` response) if you try and extract an extension that doesn’t exist, perhaps because you forgot to add the middleware or because you’re extracting the wrong type.

### [§](#using-closure-captures) Using closure captures

State can also be passed directly to handlers using closure captures:

```
use axum::{
Json,
extract::{Extension, Path},
routing::{get, post},
Router,
};
use std::sync::Arc;
use serde::Deserialize;
struct AppState {
//...
}
let shared_state = Arc::new(AppState {/*... */});
let app = Router::new()
.route(
"/users",
post({
let shared_state = Arc::clone(&amp;shared_state);
move |body| create_user(body, shared_state)
}),
)
.route(
"/users/{id}",
get({
let shared_state = Arc::clone(&amp;shared_state);
move |path| get_user(path, shared_state)
}),
);
async fn get_user(Path(user_id): Path&lt;String&gt;, state: Arc&lt;AppState&gt;) {
//...
}
async fn create_user(Json(payload): Json&lt;CreateUserPayload&gt;, state: Arc&lt;AppState&gt;) {
//...
}
#[derive(Deserialize)]
struct CreateUserPayload {
//...
}
```

The downside to this approach is that it’s the most verbose approach.

### [§](#using-task-local-variables) Using task-local variables

This also allows to share state with `IntoResponse` implementations:

```
use axum::{
extract::Request,
http::{header, StatusCode},
middleware::{self, Next},
response::{IntoResponse, Response},
routing::get,
Router,
};
use tokio::task_local;
#[derive(Clone)]
struct CurrentUser {
name: String,
}
task_local! {
pub static USER: CurrentUser;
}
async fn auth(req: Request, next: Next) -&gt; Result&lt;Response, StatusCode&gt; {
let auth_header = req
.headers()
.get(header::AUTHORIZATION)
.and_then(|header| header.to_str().ok())
.ok_or(StatusCode::UNAUTHORIZED)?;
if let Some(current_user) = authorize_current_user(auth_header).await {
// State is setup here in the middleware
Ok(USER.scope(current_user, next.run(req)).await)
} else {
Err(StatusCode::UNAUTHORIZED)
}
}
async fn authorize_current_user(auth_token: &amp;str) -&gt; Option&lt;CurrentUser&gt; {
Some(CurrentUser {
name: auth_token.to_string(),
})
}
struct UserResponse;
impl IntoResponse for UserResponse {
fn into_response(self) -&gt; Response {
// State is accessed here in the IntoResponse implementation
let current_user = USER.with(|u| u.clone());
(StatusCode::OK, current_user.name).into_response()
}
}
async fn handler() -&gt; UserResponse {
UserResponse
}
let app: Router = Router::new()
.route("/", get(handler))
.route_layer(middleware::from_fn(auth));
```

The main downside to this approach is that it only works when the async executor being used has the concept of task-local variables. The example above uses [tokio’s `task_local` macro](https://docs.rs/tokio/1/tokio/macro.task_local.html). smol does not yet offer equivalent functionality at the time of writing (see [this GitHub issue](https://github.com/smol-rs/async-executor/issues/139)).

## [§](#building-integrations-for-axum) Building integrations for axum

Libraries authors that want to provide [`FromRequest`](extract/trait.FromRequest.html), [`FromRequestParts`](extract/trait.FromRequestParts.html), or [`IntoResponse`](response/trait.IntoResponse.html) implementations should depend on the [`axum-core`](http://crates.io/crates/axum-core) crate, instead of `axum` if possible. [`axum-core`](http://crates.io/crates/axum-core) contains core types and traits and is less likely to receive breaking changes.

## [§](#required-dependencies) Required dependencies

To use axum there are a few dependencies you have to pull in as well:

```
[dependencies]
axum = &quot;&lt;latest-version&gt;&quot;
tokio = {version = &quot;&lt;latest-version&gt;&quot;, features = [&quot;full&quot;]}
tower = &quot;&lt;latest-version&gt;&quot;
```

The `"full"` feature for tokio isn’t necessary but it’s the easiest way to get started.

Tower isn’t strictly necessary either but helpful for testing. See the testing example in the repo to learn more about testing axum apps.

## [§](#examples) Examples

The axum repo contains [a number of examples](https://github.com/tokio-rs/axum/tree/main/examples) that show how to put all the pieces together.

## [§](#feature-flags) Feature flags

axum uses a set of [feature flags](https://doc.rust-lang.org/cargo/reference/features.html) to reduce the amount of compiled and optional dependencies.

The following optional features are available:

| Name | Description | Default? |
| `http1` | Enables hyper’s `http1` feature | ✔ |
| `http2` | Enables hyper’s `http2` feature |
| `json` | Enables the [`Json`](struct.Json.html) type and some similar convenience functionality | ✔ |
| `macros` | Enables optional utility macros |
| `matched-path` | Enables capturing of every request’s router path and the [`MatchedPath`](extract/struct.MatchedPath.html) extractor | ✔ |
| `multipart` | Enables parsing `multipart/form-data` requests with [`Multipart`](extract/struct.Multipart.html) |
| `original-uri` | Enables capturing of every request’s original URI and the [`OriginalUri`](extract/struct.OriginalUri.html) extractor | ✔ |
| `tokio` | Enables `tokio` as a dependency and `axum::serve`, `SSE` and `extract::connect_info` types. | ✔ |
| `tower-log` | Enables `tower` ’s `log` feature | ✔ |
| `tracing` | Log rejections from built-in extractors | ✔ |
| `ws` | Enables WebSockets support via [`extract::ws`](extract/ws/index.html) |
| `form` | Enables the `Form` extractor | ✔ |
| `query` | Enables the `Query` extractor | ✔ |
## Re-exports [§](#reexports)
`pub use http;`
## Modules [§](#modules)
[body](body/index.html) HTTP body utilities. [error_ handling](error_handling/index.html) Error handling model and utilities [extract](extract/index.html) Types and traits for extracting data from requests. [handler](handler/index.html) Async functions that can be used to handle requests. [middleware](middleware/index.html) Utilities for writing middleware [response](response/index.html) Types and traits for generating responses. [routing](routing/index.html) Routing between [`Service`](https://docs.rs/tower-service/0.3.3/x86_64-unknown-linux-gnu/tower_service/trait.Service.html) s and handlers. [serve](serve/index.html) `tokio` and (`http1` or `http2`) Serve services. [test_ helpers](test_helpers/index.html) `__private`
## Structs [§](#structs)
[Error](struct.Error.html) Errors that can happen when using axum. [Extension](struct.Extension.html) Extractor and response for extensions. [Form](struct.Form.html) `form` URL encoded extractor and response. [Json](struct.Json.html) `json` JSON Extractor / Response. [Router](struct.Router.html) The router type for composing handlers and services.
## Traits [§](#traits)
[Request Ext](trait.RequestExt.html) Extension trait that adds additional methods to [`Request`](extract/type.Request.html) . [Request Parts Ext](trait.RequestPartsExt.html) Extension trait that adds additional methods to [`Parts`](https://docs.rs/http/1.3.1/x86_64-unknown-linux-gnu/http/request/struct.Parts.html) . [Service Ext](trait.ServiceExt.html) Extension trait that adds additional methods to any [`Service`](https://docs.rs/tower-service/0.3.3/x86_64-unknown-linux-gnu/tower_service/trait.Service.html) .
## Functions [§](#functions)
[serve](fn.serve.html) `tokio` and (`http1` or `http2`) Serve the service with the supplied listener.
## Type Aliases [§](#types)
[BoxError](type.BoxError.html) Alias for a type-erased error type.
## Attribute Macros [§](#attributes)
[debug_ handler](attr.debug_handler.html) `macros` Generates better error messages when applied to handler functions. [debug_ middleware](attr.debug_middleware.html) `macros` Generates better error messages when applied to middleware functions.