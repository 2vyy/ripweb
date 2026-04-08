# Using the Fetch API

The [Fetch API](https://developer.mozilla.org/en-US/docs/Web/API/Fetch_API) provides a JavaScript interface for making HTTP requests and processing the responses.

Fetch is the modern replacement for [`XMLHttpRequest`](https://developer.mozilla.org/en-US/docs/Web/API/XMLHttpRequest): unlike `XMLHttpRequest`, which uses callbacks, Fetch is promise-based and is integrated with features of the modern web such as [service workers](https://developer.mozilla.org/en-US/docs/Web/API/Service_Worker_API) and [Cross-Origin Resource Sharing (CORS)](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/CORS).

With the Fetch API, you make a request by calling [`fetch()`](https://developer.mozilla.org/en-US/docs/Web/API/Window/fetch), which is available as a global function in both [`window`](https://developer.mozilla.org/en-US/docs/Web/API/Window) and [`worker`](https://developer.mozilla.org/en-US/docs/Web/API/WorkerGlobalScope) contexts. You pass it a [`Request`](https://developer.mozilla.org/en-US/docs/Web/API/Request) object or a string containing the URL to fetch, along with an optional argument to configure the request.

The `fetch()` function returns a [`Promise`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Promise) which is fulfilled with a [`Response`](https://developer.mozilla.org/en-US/docs/Web/API/Response) object representing the server's response. You can then check the request status and extract the body of the response in various formats, including text and JSON, by calling the appropriate method on the response.

Here's a minimal function that uses `fetch()` to retrieve some JSON data from a server:

We declare a string containing the URL and then call `fetch()`, passing the URL with no extra options.

The `fetch()` function will reject the promise on some errors, but not if the server responds with an error status like [`404`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Status/404): so we also check the response status and throw if it is not OK.

Otherwise, we fetch the response body content as [JSON](https://developer.mozilla.org/en-US/docs/Glossary/JSON) by calling the [`json()`](https://developer.mozilla.org/en-US/docs/Web/API/Response/json) method of `Response`, and log one of its values. Note that like `fetch()` itself, `json()` is asynchronous, as are all the other methods to access the response body content.

In the rest of this page we'll look in more detail at the different stages of this process.

## [Making a request](#making_a_request)

To make a request, call `fetch()`, passing in:

1. a definition of the resource to fetch. This can be any one of:
- a string containing the URL
- an object, such as an instance of [`URL`](https://developer.mozilla.org/en-US/docs/Web/API/URL) , which has a [stringifier](https://developer.mozilla.org/en-US/docs/Glossary/Stringifier) that produces a string containing the URL
- a [`Request`](https://developer.mozilla.org/en-US/docs/Web/API/Request) instance
2. optionally, an object containing options to configure the request.

In this section we'll look at some of the most commonly-used options. To read about all the options that can be given, see the [`fetch()`](https://developer.mozilla.org/en-US/docs/Web/API/Window/fetch) reference page.

### [Setting the method](#setting_the_method)

By default, `fetch()` makes a [`GET`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Methods/GET) request, but you can use the `method` option to use a different [request method](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Methods):

If the `mode` option is set to `no-cors`, then `method` must be one of `GET`, `POST` or `HEAD`.

### [Setting a body](#setting_a_body)

The request body is the payload of the request: it's the thing the client is sending to the server. You cannot include a body with `GET` requests, but it's useful for requests that send content to the server, such as [`POST`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Methods/POST) or [`PUT`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Methods/PUT) requests. For example, if you want to upload a file to the server, you might make a `POST` request and include the file as the request body.

To set a request body, pass it as the `body` option:

You can supply the body as an instance of any of the following types:

- a string
- [`ArrayBuffer`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/ArrayBuffer)
- [`TypedArray`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/TypedArray)
- [`DataView`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/DataView)
- [`Blob`](https://developer.mozilla.org/en-US/docs/Web/API/Blob)
- [`File`](https://developer.mozilla.org/en-US/docs/Web/API/File)
- [`URLSearchParams`](https://developer.mozilla.org/en-US/docs/Web/API/URLSearchParams)
- [`FormData`](https://developer.mozilla.org/en-US/docs/Web/API/FormData)
- [`ReadableStream`](https://developer.mozilla.org/en-US/docs/Web/API/ReadableStream)

Other objects are converted to strings using their `toString()` method. For example, you can use a [`URLSearchParams`](https://developer.mozilla.org/en-US/docs/Web/API/URLSearchParams) object to encode form data (see [setting headers](#setting_headers) for more information):

Note that just like response bodies, request bodies are streams, and making the request reads the stream, so if a request contains a body, you can't make it twice:

Instead, you would need to [create a clone](https://developer.mozilla.org/en-US/docs/Web/API/Request/clone) of the request before sending it:

See [Locked and disturbed streams](#locked_and_disturbed_streams) for more information.

### [Setting headers](#setting_headers)

Request headers give the server information about the request: for example, in a `POST` request, the [`Content-Type`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Content-Type) header tells the server the format of the request's body.

To set request headers, assign them to the `headers` option.

You can pass an object literal here containing `header-name: header-value` properties:

Alternatively, you can construct a [`Headers`](https://developer.mozilla.org/en-US/docs/Web/API/Headers) object, add headers to that object using [`Headers.append()`](https://developer.mozilla.org/en-US/docs/Web/API/Headers/append), then assign the `Headers` object to the `headers` option:

Compared to using plain objects, the `Headers` object provides some additional input sanitization. For example, it normalizes header names to lowercase, strips leading and trailing whitespace from header values, and prevents certain headers from being set. Many headers are set automatically by the browser and can't be set by a script: these are called [Forbidden request headers](https://developer.mozilla.org/en-US/docs/Glossary/Forbidden_request_header). If the [`mode`](https://developer.mozilla.org/en-US/docs/Web/API/Request/mode) option is set to `no-cors`, then the set of permitted headers is further restricted.

### [Sending data in a GET request](#sending_data_in_a_get_request)

`GET` requests don't have a body, but you can still send data to the server by appending it to the URL as a query string. This is a common way to send form data to the server. You can do this by using [`URLSearchParams`](https://developer.mozilla.org/en-US/docs/Web/API/URLSearchParams) to encode the data, and then appending it to the URL:

### [Making cross-origin requests](#making_cross-origin_requests)

Whether a request can be made cross-origin or not is determined by the value of the [`RequestInit.mode`](https://developer.mozilla.org/en-US/docs/Web/API/RequestInit) option. This may take one of three values: `cors`, `same-origin`, or `no-cors`.

- For fetch requests the default value of `mode` is `cors`, meaning that if the request is cross-origin then it will use the [Cross-Origin Resource Sharing (CORS)](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/CORS) mechanism. This means that:
- if the request is a [simple request](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/CORS) , then the request will always be sent, but the server must respond with the correct [`Access-Control-Allow-Origin`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Access-Control-Allow-Origin) header or the browser will not share the response with the caller.
- if the request is not a simple request, then the browser will send a [preflighted request](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/CORS) to check that the server understands CORS and allows the request, and the real request will not be sent unless the server responds to the preflighted request with the appropriate CORS headers.
- Setting `mode` to `same-origin` disallows cross-origin requests completely.
- Setting `mode` to `no-cors` disables CORS for cross-origin requests. This restricts the headers that may be set, and restricts methods to GET, HEAD, and POST. The response is *opaque*, meaning that its headers and body are not available to JavaScript. Most of the time a website should not use `no-cors`: the main application of it is for certain service worker use cases.

See the reference documentation for [`RequestInit.mode`](https://developer.mozilla.org/en-US/docs/Web/API/RequestInit) for more details.

### [Including credentials](#including_credentials)

In the context of the Fetch API, a credential is an extra piece of data sent along with the request that the server may use to authenticate the user. All the following items are considered to be credentials:

- HTTP cookies
- [TLS](https://developer.mozilla.org/en-US/docs/Glossary/TLS) client certificates
- The [`Authorization`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Authorization) and [`Proxy-Authorization`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Proxy-Authorization) headers.

By default, credentials are only included in same-origin requests. To customize this behavior, as well as to control whether the browser respects any **`Set-Cookie`** response headers, set the [`credentials`](https://developer.mozilla.org/en-US/docs/Web/API/RequestInit) option, which can take one of the following three values:

- `omit` : never send credentials in the request or include credentials in the response.
- `same-origin` (the default): only send and include credentials for same-origin requests.
- `include` : always include credentials, even cross-origin.

Note that if a cookie's [`SameSite`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Set-Cookie) attribute is set to `Strict` or `Lax`, then the cookie will not be sent cross-site, even if `credentials` is set to `include`.

Including credentials in cross-origin requests can make a site vulnerable to [CSRF](https://developer.mozilla.org/en-US/docs/Glossary/CSRF) attacks, so even if `credentials` is set to `include`, the server must also agree to their inclusion by including the [`Access-Control-Allow-Credentials`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Access-Control-Allow-Credentials) header in its response. Additionally, in this situation the server must explicitly specify the client's origin in the [`Access-Control-Allow-Origin`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Access-Control-Allow-Origin) response header (that is, `*` is not allowed).

This means that if `credentials` is set to `include` and the request is cross-origin, then:

- If the request is a [simple request](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/CORS), then the request will be sent with credentials, but the server must set the [`Access-Control-Allow-Credentials`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Access-Control-Allow-Credentials) and [`Access-Control-Allow-Origin`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Access-Control-Allow-Origin) response headers, or the browser will return a network error to the caller. If the server does set the correct headers, then the response, including credentials, will be delivered to the caller.
- If the request is not a simple request, then the browser will send a [preflighted request](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/CORS) without credentials, and the server must set the [`Access-Control-Allow-Credentials`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Access-Control-Allow-Credentials) and [`Access-Control-Allow-Origin`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Access-Control-Allow-Origin) response headers, or the browser will return a network error to the caller. If the server does set the correct headers, then the browser will follow up with the real request, including credentials, and will deliver the real response, including credentials, to the caller.

### [Creating a `Request` object](#creating_a_request_object)

The [`Request()`](https://developer.mozilla.org/en-US/docs/Web/API/Request/Request) constructor takes the same arguments as `fetch()` itself. This means that instead of passing options into `fetch()`, you can pass the same options to the `Request()` constructor, and then pass that object to `fetch()`.

For example, we can make a POST request by passing options into `fetch()` using code like this:

However, we could rewrite this to pass the same arguments to the `Request()` constructor:

This also means that you can create a request from another request, while changing some of its properties using the second argument:

## [Canceling a request](#canceling_a_request)

To make a request cancelable, create an [`AbortController`](https://developer.mozilla.org/en-US/docs/Web/API/AbortController), and assign its [`AbortSignal`](https://developer.mozilla.org/en-US/docs/Web/API/AbortSignal) to the request's `signal` property.

To cancel the request, call the controller's [`abort()`](https://developer.mozilla.org/en-US/docs/Web/API/AbortController/abort) method. The `fetch()` call will reject the promise with an `AbortError` exception.

If the request is aborted after the `fetch()` call has been fulfilled but before the response body has been read, then attempting to read the response body will reject with an `AbortError` exception.

## [Handling the response](#handling_the_response)

As soon as the browser has received the response status and headers from the server (and potentially before the response body itself has been received), the promise returned by `fetch()` is fulfilled with a [`Response`](https://developer.mozilla.org/en-US/docs/Web/API/Response) object.

### [Checking response status](#checking_response_status)

The promise returned by `fetch()` will reject on some errors, such as a network error or a bad scheme. However, if the server responds with an error like [`404`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Status/404), then `fetch()` fulfills with a `Response`, so we have to check the status before we can read the response body.

The [`Response.status`](https://developer.mozilla.org/en-US/docs/Web/API/Response/status) property tells us the numerical status code, and the [`Response.ok`](https://developer.mozilla.org/en-US/docs/Web/API/Response/ok) property returns `true` if the status is in the [200 range](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Status).

A common pattern is to check the value of `ok` and throw if it is `false`:

### [Checking the response type](#checking_the_response_type)

Responses have a [`type`](https://developer.mozilla.org/en-US/docs/Web/API/Response/type) property that can be one of the following:

- `basic` : the request was a same-origin request.
- `cors` : the request was a cross-origin CORS request.
- `opaque` : the request was a cross-origin simple request made with the `no-cors` mode.
- `opaqueredirect` : the request set the `redirect` option to `manual` , and the server returned a [redirect status](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Status) .

The type determines the possible contents of the response, as follows:

- Basic responses exclude response headers from the [Forbidden response header name](https://developer.mozilla.org/en-US/docs/Glossary/Forbidden_response_header_name) list.
- CORS responses include only response headers from the [CORS-safelisted response header](https://developer.mozilla.org/en-US/docs/Glossary/CORS-safelisted_response_header) list.
- Opaque responses and opaque redirect responses have a `status` of `0`, an empty header list, and a `null` body.

### [Checking headers](#checking_headers)

Just like the request, the response has a [`headers`](https://developer.mozilla.org/en-US/docs/Web/API/Response/headers) property which is a [`Headers`](https://developer.mozilla.org/en-US/docs/Web/API/Headers) object, and this contains any response headers that are exposed to scripts, subject to the exclusions made based on the response type.

A common use case for this is to check the content type before trying to read the body:

### [Reading the response body](#reading_the_response_body)

The `Response` interface provides a number of methods to retrieve the entire body contents in a variety of different formats:

- [`Response.arrayBuffer()`](https://developer.mozilla.org/en-US/docs/Web/API/Response/arrayBuffer)
- [`Response.blob()`](https://developer.mozilla.org/en-US/docs/Web/API/Response/blob)
- [`Response.formData()`](https://developer.mozilla.org/en-US/docs/Web/API/Response/formData)
- [`Response.json()`](https://developer.mozilla.org/en-US/docs/Web/API/Response/json)
- [`Response.text()`](https://developer.mozilla.org/en-US/docs/Web/API/Response/text)

These are all asynchronous methods, returning a [`Promise`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Promise) which will be fulfilled with the body content.

In this example, we fetch an image and read it as a [`Blob`](https://developer.mozilla.org/en-US/docs/Web/API/Blob), which we can then use to create an object URL:

The method will throw an exception if the response body is not in the appropriate format: for example, if you call `json()` on a response that can't be parsed as JSON.

### [Streaming the response body](#streaming_the_response_body)

Request and response bodies are actually [`ReadableStream`](https://developer.mozilla.org/en-US/docs/Web/API/ReadableStream) objects, and whenever you read them, you're streaming the content. This is good for memory efficiency, because the browser doesn't have to buffer the entire response in memory before the caller retrieves it using a method like `json()`.

This also means that the caller can process the content incrementally as it is received.

For example, consider a `GET` request that fetches a large text file and processes it in some way, or displays it to the user:

If we use [`Response.text()`](https://developer.mozilla.org/en-US/docs/Web/API/Response/text), as above, we must wait until the whole file has been received before we can process any of it.

If we stream the response instead, we can process chunks of the body as they are received from the network:

In this example, we [iterate asynchronously](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Statements/for-await...of) over the stream, processing each chunk as it arrives.

Note that when you access the body directly like this, you get the raw bytes of the response and must transform it yourself. In this case we call [`ReadableStream.pipeThrough()`](https://developer.mozilla.org/en-US/docs/Web/API/ReadableStream/pipeThrough) to pipe the response through a [`TextDecoderStream`](https://developer.mozilla.org/en-US/docs/Web/API/TextDecoderStream), which decodes the UTF-8-encoded body data as text.

### [Processing a text file line by line](#processing_a_text_file_line_by_line)

In the example below, we fetch a text resource and process it line by line, using a regular expression to look for line endings. For simplicity, we assume the text is UTF-8, and don't handle fetch errors:

### [Locked and disturbed streams](#locked_and_disturbed_streams)

The consequences of request and response bodies being streams are that:

- if a reader has been attached to a stream using `ReadableStream.getReader()` , then the stream is *locked* , and nothing else can read the stream.
- if any content has been read from the stream, then the stream is *disturbed* , and nothing else can read from the stream.

This means it's not possible to read the same response (or request) body more than once:

If you do need to read the body more than once, you must call [`Response.clone()`](https://developer.mozilla.org/en-US/docs/Web/API/Response/clone) before reading the body:

This is a common pattern when [implementing an offline cache with service workers](https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps/Guides/Caching). The service worker wants to return the response to the app, but also to cache the response. So it clones the response, returns the original, and caches the clone:

## [See also](#see_also)

- [Service Worker API](https://developer.mozilla.org/en-US/docs/Web/API/Service_Worker_API)
- [Streams API](https://developer.mozilla.org/en-US/docs/Web/API/Streams_API)
- [CORS](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/CORS)
- [HTTP](https://developer.mozilla.org/en-US/docs/Web/HTTP)
- [Fetch examples on GitHub](https://github.com/mdn/dom-examples/tree/main/fetch)

## Help improve MDN
[Learn how to contribute](https://developer.mozilla.org/en-US/docs/MDN/Community/Getting_started)
This page was last modified on Aug 20, 2025 by [MDN contributors](https://developer.mozilla.org/en-US/docs/Web/API/Fetch_API/Using_Fetch/contributors.txt).
[View this page on GitHub](https://github.com/mdn/content/blob/main/files/en-us/web/api/fetch_api/using_fetch/index.md?plain=1) • [Report a problem with this content](https://github.com/mdn/content/issues/new?template=page-report.yml&amp%3Bmdn-url=https%3A%2F%2Fdeveloper.mozilla.org%2Fen-US%2Fdocs%2FWeb%2FAPI%2FFetch_API%2FUsing_Fetch&amp%3Bmetadata=%3C%21--+Do+not+make+changes+below+this+line+--%3E%0A%3Cdetails%3E%0A%3Csummary%3EPage+report+details%3C%2Fsummary%3E%0A%0A*+Folder%3A+%60en-us%2Fweb%2Fapi%2Ffetch_api%2Fusing_fetch%60%0A*+MDN+URL%3A+https%3A%2F%2Fdeveloper.mozilla.org%2Fen-US%2Fdocs%2FWeb%2FAPI%2FFetch_API%2FUsing_Fetch%0A*+GitHub+URL%3A+https%3A%2F%2Fgithub.com%2Fmdn%2Fcontent%2Fblob%2Fmain%2Ffiles%2Fen-us%2Fweb%2Fapi%2Ffetch_api%2Fusing_fetch%2Findex.md%0A*+Last+commit%3A+https%3A%2F%2Fgithub.com%2Fmdn%2Fcontent%2Fcommit%2Ffe1d7fb9b67ce826c4a748ce00e7b35ac4a54c7f%0A*+Document+last+modified%3A+2025-08-20T18%3A33%3A20.000Z%0A%0A%3C%2Fdetails%3E)