4097 tokens
Crate axum Source[1] axum is a web application framework that focuses on ergonomics and modularity.

High-level features  
- Route requests to handlers with a macro-free API.
- Declaratively parse requests using extractors.
- Simple and predictable error handling model.
- Generate responses with minimal boilerplate.
- Take full advantage of the [`tower`](https://crates.io/crates/tower) and [`tower-http`](https://crates.io/crates/tower-http) ecosystem of middleware, services, and utilities.

In particular, the last point is what sets `axum` apart from other frameworks. `axum` doesn’t have its own middleware system but instead uses [`tower::Service`](https://docs.rs/tower-service/0.3.3/x86_64-unknown-linux-gnu/tower_service/trait.Service.html). This means `axum` gets timeouts, tracing, compression, authorization, and more, for free. It also enables you to share middleware with applications written using [`hyper`](http://crates.io/crates/hyper) or [`tonic`](http://crates.io/crates/tonic).

## Compatibility

axum is designed to work with [tokio](https://docs.rs/tokio/1.47.1/x86_64-unknown-linux-gnu/tokio/index.html) and [hyper](https://docs.rs/hyper/1.7.0/x86_64-unknown-linux-gnu/hyper/index.html). Runtime and transport layer independence is not a goal, at least for the time being.

## Example  

The "Hello, World!" of axum is:


```rust
use axum::{
    routing::get,
    Router,
};

#[tokio::main]
async fn main() {
    // build our application with a single route
    let app = Router::new().route("/", get(|| async { "Hello, World!" }));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

Note using `#[tokio::main]` requires you enable tokio’s `macros` and `rt-multi-thread` features or just `full` to enable all features (`cargo add tokio --features macros,rt-multi-thread`).

## Routing

Router is used to set up which paths go to which services:

```rust
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

  

See [`Router`](https://docs.rs/axum/latest/axum/struct.Router.html) for more details on routing.

## Handlers

In axum a “handler” is an async function that accepts zero or more [“extractors”](https://docs.rs/axum/latest/axum/extract/index.html) as arguments and returns something that can be converted [into a response](https://docs.rs/axum/latest/axum/response/index.html).

Handlers are where your application logic lives and axum applications are built by routing between handlers.

See [`handler`](https://docs.rs/axum/latest/axum/handler/index.html) for more details on handlers.

## Extractors

  

An extractor is a type that implements [`FromRequest`](https://docs.rs/axum/latest/axum/extract/trait.FromRequest.html) or [`FromRequestParts`](https://docs.rs/axum/latest/axum/extract/trait.FromRequestParts.html). Extractors are how you pick apart the incoming request to get the parts your handler needs. 

```rust
use axum::extract::{Path, Query, Json};
use std::collections::HashMap;

// `Path` gives you the path parameters and deserializes them.
async fn path(Path(user_id): Path<u32>) {}

// `Query` gives you the query parameters and deserializes them.
async fn query(Query(params): Query<HashMap<String, String>>) {}

// Buffer the request body and deserialize it as JSON into a
// `serde_json::Value`. `Json` supports any type that implements
// `serde::Deserialize`.
async fn json(Json(payload): Json<serde_json::Value>) {}
```

See [`extract`](https://docs.rs/axum/latest/axum/extract/index.html) for more details on extractors.


## Responses


Anything that implements [`IntoResponse`](https://docs.rs/axum/latest/axum/response/trait.IntoResponse.html) can be returned from handlers.

```rust
use axum::{
    body::Body,
    routing::get,
    response::Json,
    Router,
};
use serde_json::{Value, json};

// `&'static str` becomes a `200 OK` with `content-type: text/plain; charset=utf-8`
async fn plain_text() -> &'static str {
    "foo"
}

// `Json` gives a content-type of `application/json` and works with any type
// that implements `serde::Serialize`
async fn json() -> Json<Value> {
    Json(json!({ "data": 42 }))
}

let app = Router::new()
    .route("/plain_text", get(plain_text))
    .route("/json", get(json));
```

See [`response`](https://docs.rs/axum/latest/axum/response/index.html) for more details on building responses.
  

## Error handling

  

axum aims to have a simple and predictable error handling model. That means it is simple to convert errors into responses and you are guaranteed that all errors are handled.

See [`error_handling`](https://docs.rs/axum/latest/axum/error_handling/index.html") for more details on axum’s error handling model and how to handle errors gracefully.

  

## Middleware

  

There are several different ways to write middleware for axum. See [`middleware`](https://docs.rs/axum/latest/axum/middleware/index.html) for more details.

  

## Sharing state with handlers

  
It is common to share some state between handlers. For example, a pool of database connections or clients to other services may need to be shared.

The four most common ways of doing that are:

-   Using the [`State`](https://docs.rs/axum/latest/axum/extract/struct.State.html) extractor
-   Using request extensions
-   Using closure captures
-   Using task-local variables

  

## Using the [`State`](https://docs.rs/axum/latest/axum/extract/struct.State.html) extractor

  

```rust
use axum::{
    extract::State,
    routing::get,
    Router,
};
use std::sync::Arc;

struct AppState {
    // ...
}

let shared_state = Arc::new(AppState { /* ... */ });

let app = Router::new()
    .route("/", get(handler))
    .with_state(shared_state);

async fn handler(
    State(state): State<Arc<AppState>>,
) {
    // ...
}
```

  

You should prefer using [`State`](https://docs.rs/axum/latest/axum/extract/struct.State.html) if possible since it’s more type safe. The downside is that it’s less dynamic than task-local variables and request extensions.

See [`State`](https://docs.rs/axum/latest/axum/extract/struct.State.html) for more details about accessing state.

## Using request extensions:
Another way to share state with handlers is using [`Extension`](https://docs.rs/axum/latest/axum/struct.Extension.html) as layer and extractor:
  

```rust
use axum::{
    extract::Extension,
    routing::get,
    Router,
};
use std::sync::Arc;

struct AppState {
    // ...
}

let shared_state = Arc::new(AppState { /* ... */ });

let app = Router::new()
    .route("/", get(handler))
    .layer(Extension(shared_state));

async fn handler(
    Extension(state): Extension<Arc<AppState>>,
) {
    // ...
}
```

The downside to this approach is that you’ll get runtime errors (specifically a `500 Internal Server Error` response) if you try and extract an extension that doesn’t exist, perhaps because you forgot to add the middleware o
r because you’re extracting the wrong type.

  

## Using closure captures

State can also be passed directly to handlers using closure captures:

```rust
use axum::{
    Json,
    extract::{Extension, Path},
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use serde::Deserialize;

struct AppState {
    // ...
}

let shared_state = Arc::new(AppState { /* ... */ });

let app = Router::new()
    .route(
        "/users",
        post({
            let shared_state = Arc::clone(&shared_state);
            move |body| create_user(body, shared_state)
        }),
    )
    .route(
        "/users/{id}",
        get({
            let shared_state = Arc::clone(&shared_state);
            move |path| get_user(path, shared_state)
        }),
    );

async fn get_user(Path(user_id): Path<String>, state: Arc<AppState>) {
    // ...
}

async fn create_user(Json(payload): Json<CreateUserPayload>, state: Arc<AppState>) {
    // ...
}

#[derive(Deserialize)]
struct CreateUserPayload {
    // ...
}
```
The downside to this approach is that it’s the most verbose approach.

## Using task-local variables

This also allows to share state with `IntoResponse` implementations:

```rust
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

async fn auth(req: Request, next: Next) -> Result<Response, StatusCode> {
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
async fn authorize_current_user(auth_token: &str) -> Option<CurrentUser> {
    Some(CurrentUser {
        name: auth_token.to_string(),
    })
}

struct UserResponse;

impl IntoResponse for UserResponse {
    fn into_response(self) -> Response {
        // State is accessed here in the IntoResponse implementation
        let current_user = USER.with(|u| u.clone());
        (StatusCode::OK, current_user.name).into_response()
    }
}

async fn handler() -> UserResponse {
    UserResponse
}

let app: Router = Router::new()
    .route("/", get(handler))
    .route_layer(middleware::from_fn(auth));
```
The main downside to this approach is that it only works when the async executor being used has the concept of task-local variables. The example above uses [tokio’s `task_local` macro](https://docs.rs/tokio/1/tokio/macro.task_local.html). smol does not yet offer equivalent functionality at the time of writing (see [this GitHub issue](https://github.com/smol-rs/async-executor/issues/139)).
## Building integrations for axum
Libraries authors that want to provide [`FromRequest`](https://docs.rs/axum/latest/axum/extract/trait.FromRequest.html"), [`FromRequestParts`](https://docs.rs/axum/latest/axum/extract/trait.FromRequestParts.html), or [`IntoResponse`](https://docs.rs/axum/latest/axum/response/trait.IntoResponse.html) implementations should depend on the [`axum-core`](http://crates.io/crates/axum-core) crate, instead of `axum` if possible. [`axum-core`](http://crates.io/crates/axum-core) contains core types and traits and is less likely to receive breaking changes.

## Required dependencies
To use axum there are a few dependencies you have to pull in as well:

```rust
[dependencies]
axum = "<latest-version>"
tokio = { version = "<latest-version>", features = ["full"] }
tower = "<latest-version>"
```

The `"full"` feature for tokio isn’t necessary but it’s the easiest way to get started.

Tower isn’t strictly necessary either but helpful for testing. See the testing example in the repo to learn more about testing axum apps.

## Examples
The axum repo contains [a number of examples](https://github.com/tokio-rs/axum/tree/main/examples) that show how to put all the pieces together.

## Feature flags
axum uses a set of [feature flags](https://doc.rust-lang.org/cargo/reference/features.html#the-features-section) to reduce the amount of compiled and optional dependencies.

The following optional features are available:
```markdown
| Name | Description | Default? |
| `http1` | Enables hyper’s `http1` feature | ✔ |
| `http2` | Enables hyper’s `http2` feature |  |
| `json` | Enables the [`Json`](https://docs.rs/axum/latest/axum/struct.Json.html) type and some similar convenience functionality | ✔ |
| `macros` | Enables optional utility macros |  |
| `multipart` | Enables parsing `multipart/form-data` requests with [`Multipart`](https://docs.rs/axum/latest/axum/extract/struct.Multipart.html) | ✔ |
| `original-uri` | Enables capturing of every request’s original URI and the [`OriginalUri`](https://docs.rs/axum/latest/axum/extract/struct.OriginalUri.html) extractor |  |
| original-uri | Enables capturing of every request’s original URI and the [`OriginalUri`](https://docs.rs/axum/latest/axum/extract/struct.OriginalUri.html) extractor | ✔ |
| `tokio` | Enables `tokio` as a dependency and `axum::serve`, `SSE` and `extract::connect_info` types. | ✔ |
| `tower-log` | Enables `tower`’s `log` feature | ✔ |
| `tracing` | Log rejections from built-in extractors | ✔ |
| `ws` | Enables WebSockets support via [`extract::ws`](https://docs.rs/axum/latest/axum/extract/ws/index.html) |  |
| `form` | Enables the `Form` extractor | ✔ |
| `query` | Enables the `Query` extractor | ✔ |
```
## Re-exports
`pub use [http](https://docs.rs/http/1.3.1/x86_64-unknown-linux-gnu/http/index.html);`

##  Modules
| [body](https://docs.rs/axum/latest/axum/body/index.html "mod axum::body") | HTTP body utilities. |
| [error_handling](https://docs.rs/axum/latest/axum/error_handling/index.html "mod axum::error_handling") | Error handling model and utilities |
| [extract](https://docs.rs/axum/latest/axum/extract/index.html "mod axum::extract") | Types and traits for extracting data from requests. |
| [handler](https://docs.rs/axum/latest/axum/handler/index.html "mod axum::handler") | Async functions that can be used to handle requests. |
| [middleware](https://docs.rs/axum/latest/axum/middleware/index.html "mod axum::middleware") | Utilities for writing middleware |
| [response](https://docs.rs/axum/latest/axum/response/index.html "mod axum::response") | Types and traits for generating responses. |
| [routing](https://docs.rs/axum/latest/axum/routing/index.html) | Routing between [`Service`](https://docs.rs/tower-service/0.3.3/x86_64-unknown-linux-gnu/tower_service/trait.Service.html)s and handlers. |
| [serve](https://docs.rs/axum/latest/axum/serve/index.html)`tokio` and (`http1` or `http2`) | Serve services. |
| [test_helpers](https://docs.rs/axum/latest/axum/test_helpers/index.html) `__private` | |

## Structs

| [Error](https://docs.rs/axum/latest/axum/struct.Error.html) | Errors that can happen when using axum. |
| [Extension](https://docs.rs/axum/latest/axum/struct.Extension.html) | Extractor and response for extensions. |
| [Form](https://docs.rs/axum/latest/axum/struct.Form.html ) `form` | URL encoded extractor and response. |
| [Json](https://docs.rs/axum/latest/axum/struct.Json.html) `json` | JSON Extractor / Response. |
| [Router](https://docs.rs/axum/latest/axum/struct.Router.html) | The router type for composing handlers and services. |

## Traits

| [RequestExt](https://docs.rs/axum/latest/axum/trait.RequestExt.html) | Extension trait that adds additional methods to [`Request`](https://docs.rs/axum/latest/axum/extract/type.Request.html). |

| [RequestPartsExt](https://docs.rs/axum/latest/axum/trait.RequestPartsExt.html) | Extension trait that adds additional methods to [`Parts`](https://docs.rs/http/1.3.1/x86_64-unknown-linux-gnu/http/request/struct.Parts.html). |

| [ServiceExt](https://docs.rs/axum/latest/axum/trait.ServiceExt.html) | Extension trait that adds additional methods to any [`Service`](https://docs.rs/tower-service/0.3.3/x86_64-unknown-linux-gnu/tower_service/trait.Service.html). |

## Functions

| [serve](https://docs.rs/axum/latest/axum/fn.serve.html) `tokio` and (`http1` or `http2`) | Serve the service with the supplied listener. |

## Type Aliases

| [BoxError](https://docs.rs/axum/latest/axum/type.BoxError.html) | Alias for a type-erased error type. |

## Attribute Macros

| [debug_handler](https://docs.rs/axum/latest/axum/attr.debug_handler.html) `macros` | Generates better error messages when applied to handler functions. |
| [debug_middleware](https://docs.rs/axum/latest/axum/attr.debug_middleware.html) `macros` | Generates better error messages when applied to middleware functions. |

[1]: docs.rs/axum/latest/src/axum/lib.rs.html
[2]: 
