Response APIs
-------------
**Base**
1) `status`     - set response code
2) `header.set` - set the headers
3) `write`      - Transfer-Encoding: chunked
4) `end`        - To write last zero chunk
5) `send`       - write string; charset(utf-8); content-type(default - text/html)

**Wrapper**
6) `json`       - stringify json and write
7) `sendFile`   - writes the content of the file. `Content-Type` header set based file extention (default - application/octet-stream)
8) `download`   - Same as `sendFile`, but adds `Content-Disposition: attachment` header
9) `redirect`   - `302` or `301` redirection based on `Location` header
10) `type`      - set content type