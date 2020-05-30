HttpServer APIs
---------------
1) `static`     - for serving static files from a given path

HttpRequest APIs
----------------
1) `body`       - raw HTTP body
2) `url`        - URL of the HTTP request
3) `params`     - URL path params
4) `query`      - URL queries
5) `accepts`    - Only accept the request if the accept-type matches any of given types.

HttpRequestBody APIs
--------------------
1) `json`       - parses JSON in the HTTP body
2) `urlencoded` - parses urlencoded values in the HTTP body
3) `cookie`     - returns cookie from the request header
