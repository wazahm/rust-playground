HttpServer APIs
---------------
1) `static`     - for serving static files from a given path

HttpRequest APIs
----------------
1) `body`       - raw HTTP body
2) `url`        - URL of the HTTP request
3) `params`     - URL path params
4) `query`      - URL queries
5) `cookie`     - returns cookie from the request header

HttpRequestBody APIs
--------------------
1) `json`       - parses JSON in the HTTP body
2) `urlencoded` - parses urlencoded values in the HTTP body
