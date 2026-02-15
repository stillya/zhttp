# zhttp — HTTP extension for Zed

> Fork of [tie304/zed-http](https://github.com/tie304/zed-http)

## Overview

The `zhttp` extension provides syntax highlighting, LSP completions, and runnable HTTP requests for `.http` files in the Zed editor. This extension aims to replicate and eventually expand upon the functionality similar to the HTTP request capabilities seen in JetBrains editors, as described [here](https://github.com/JetBrains/http-request-in-editor-spec/blob/master/spec.md).

## Features

- Syntax highlighting
- LSP completions
- Execute requests from the editor

## Install

Install the `zhttp` extension from the Zed extension registry. The `zhttp-lsp` binary is installed automatically when you first open an `.http` file.

## Usage

Create a `.http` file and write your requests separated by `###`:

```http
### Get a post
GET https://jsonplaceholder.typicode.com/posts/1
Accept: application/json

###

### Create a new post
POST https://jsonplaceholder.typicode.com/posts
Content-Type: application/json

{
  "title": "foo",
  "body": "bar",
  "userId": 1
}
```

Click the run button next to any request to execute it. The task label shows the method and URL dynamically (e.g., `GET https://jsonplaceholder.typicode.com/posts/1`).
