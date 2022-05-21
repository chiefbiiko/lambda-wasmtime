// @ts-ignore
import { Console } from "as-wasi";
import { Method, RequestBuilder, Response } from "@deislabs/wasi-experimental-http";

export function _start(): void {
  let body = String.UTF8.encode("testing the body");
  let res = new RequestBuilder("https://postman-echo.com/post")
    .header("Content-Type", "text/plain")
    .header("abc", "def")
    .method(Method.POST)
    .body(body)
    .send();

  check(res, 200, "content-type");
  res.close();
}

function check(
  res: Response,
  expectedStatus: u32,
  expectedHeader: string
): void {
  if (res.status != expectedStatus) {
    Console.write(
      "expected status " +
        expectedStatus.toString() +
        " got " +
        res.status.toString()
    );
    abort();
  }

  let headerValue = res.headerGet(expectedHeader);
  if (!headerValue) {
    abort();
  }

  let headers = res.headerGetAll();
  if (headers.size == 0) {
    abort();
  }
}
