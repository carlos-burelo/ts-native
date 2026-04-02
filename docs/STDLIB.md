# TSN Standard Library Reference

The TSN standard library is imported via `"std:module"` specifiers.

```tsn
import { Math } from "std:math"
import { readFile } from "std:fs"
```

---

## Table of Contents

- [std:async](#stdasync)
- [std:console](#stdconsole)
- [std:collections](#stdcollections)
- [std:crypto](#stdcrypto)
- [std:dispose](#stddispose)
- [std:fs](#stdfs)
- [std:http](#stdhttp)
- [std:io](#stdio)
- [std:json](#stdjson)
- [std:math](#stdmath)
- [std:path](#stdpath)
- [std:reflect](#stdreflect)
- [std:result](#stdresult)
- [std:sys](#stdsys)
- [std:test](#stdtest)
- [std:time](#stdtime)
- [Builtins (no import)](#builtins)

---

## std:async

Async utilities and task spawning.

```tsn
import { spawn, sleep } from "std:async"
```

| Export  | Signature                             | Description                              |
|---------|---------------------------------------|------------------------------------------|
| `spawn` | `spawn(fn, ...args): Future<T>`       | Run async function as concurrent task    |
| `sleep` | `sleep(ms: int): Future<void>`        | Suspend for `ms` milliseconds            |

**Example:**

```tsn
import { spawn, sleep } from "std:async"

async function worker(id: int): str {
    await sleep(100)
    return "done:" + id
}

const t1 = spawn(worker, 1)
const t2 = spawn(worker, 2)
print(await t1)  // done:1
print(await t2)  // done:2
```

---

## std:console

Enhanced console output beyond the global `print`.

```tsn
import { log, warn, error, info } from "std:console"
```

| Export  | Signature                  | Description                |
|---------|----------------------------|----------------------------|
| `log`   | `log(...args): void`       | Standard output            |
| `warn`  | `warn(...args): void`      | Warning output             |
| `error` | `error(...args): void`     | Error output (stderr)      |
| `info`  | `info(...args): void`      | Info output                |

---

## std:collections

Generic data structures and range utilities.

```tsn
import { Range } from "std:collections"
```

| Export  | Description                |
|---------|----------------------------|
| `Range` | Integer range type         |

**Range:**

```tsn
import { Range } from "std:collections"

const r = Range.from(1, 10)         // 1..10 (exclusive end)
const ri = Range.fromInclusive(1, 10) // 1..=10

r.contains(5)   // true
r.toArray()     // [1, 2, 3, 4, 5, 6, 7, 8, 9]
r.step(2)       // Range(1, 10, step=2)
```

---

## std:crypto

Cryptographic primitives.

```tsn
import { sha256, uuid, base64Encode } from "std:crypto"
```

| Export          | Signature                                    | Description                  |
|-----------------|----------------------------------------------|------------------------------|
| `sha256`        | `sha256(input: str): str`                    | SHA-256 hex digest           |
| `sha512`        | `sha512(input: str): str`                    | SHA-512 hex digest           |
| `hmac`          | `hmac(key: str, data: str, alg: str): str`   | HMAC digest                  |
| `base64Encode`  | `base64Encode(input: str): str`              | Base64 encode                |
| `base64Decode`  | `base64Decode(input: str): str`              | Base64 decode                |
| `randomBytes`   | `randomBytes(n: int): str`                   | Random hex string            |
| `uuid`          | `uuid(): str`                                | Generate UUID v4             |

**Example:**

```tsn
import { sha256, uuid } from "std:crypto"

const hash = sha256("hello world")
print(hash)       // b94d27b99...
print(uuid())     // 550e8400-e29b-41d4-a716-446655440000
```

---

## std:dispose

Interfaces for deterministic resource management.

```tsn
import { Disposable, AsyncDisposable } from "std:dispose"
```

| Export           | Description                           |
|------------------|---------------------------------------|
| `Disposable`     | Sync disposable (`dispose(): void`)   |
| `AsyncDisposable`| Async disposable (`disposeAsync(): Future<void>`) |

**Example:**

```tsn
import { Disposable } from "std:dispose"

class DatabaseConnection implements Disposable {
    constructor() {
        print("DB connected")
    }

    query(sql: str): str {
        return "results"
    }

    dispose() {
        print("DB disconnected")
    }
}

using db = new DatabaseConnection()
const results = db.query("SELECT * FROM users")
// dispose() called automatically at block end
```

---

## std:fs

File system operations (async).

```tsn
import { readFile, writeFile, exists } from "std:fs"
```

| Export       | Signature                                    | Description                      |
|--------------|----------------------------------------------|----------------------------------|
| `readFile`   | `readFile(path: str): Future<str>`           | Read file as string              |
| `writeFile`  | `writeFile(path: str, data: str): Future<void>` | Write string to file          |
| `appendFile` | `appendFile(path: str, data: str): Future<void>` | Append to file               |
| `exists`     | `exists(path: str): Future<bool>`            | Check if path exists             |
| `remove`     | `remove(path: str): Future<void>`            | Delete file or directory         |
| `mkdir`      | `mkdir(path: str): Future<void>`             | Create directory (recursive)     |
| `readDir`    | `readDir(path: str): Future<str[]>`          | List directory entries           |
| `copy`       | `copy(from: str, to: str): Future<void>`     | Copy file                        |
| `rename`     | `rename(from: str, to: str): Future<void>`   | Move/rename file                 |

**Example:**

```tsn
import { readFile, writeFile, exists } from "std:fs"

async function main() {
    if await exists("data.txt") {
        const content = await readFile("data.txt")
        print(content)
    } else {
        await writeFile("data.txt", "hello")
    }
}

await main()
```

---

## std:http

HTTP client and server.

```tsn
import { get, post, server } from "std:http"
```

### HTTP Client

| Export   | Signature                                      | Description     |
|----------|------------------------------------------------|-----------------|
| `get`    | `get(url: str): Future<Response>`             | HTTP GET        |
| `post`   | `post(url: str, body: str): Future<Response>` | HTTP POST       |

**Response class:**

| Member     | Type     | Description              |
|------------|----------|--------------------------|
| `status`   | `int`    | HTTP status code         |
| `body`     | `str`    | Response body as string  |
| `headers`  | `unknown`| Response headers         |

### HTTP Server

```tsn
import { server } from "std:http"

server.listen(8080, (req, res) => {
    res.status(200)
    res.send("Hello from TSN!")
})
```

---

## std:io

Stdin/stdout stream access.

```tsn
import { readLine, readAll, write, print, flush } from "std:io"
```

| Export      | Signature                        | Description              |
|-------------|----------------------------------|--------------------------|
| `readLine`  | `readLine(): Future<str\|null>`  | Read one line from stdin |
| `readAll`   | `readAll(): Future<str>`         | Read all stdin           |
| `write`     | `write(s: str): void`            | Write to stdout          |
| `print`     | `print(s: str): void`            | Write + newline          |
| `flush`     | `flush(): void`                  | Flush stdout             |

---

## std:json

JSON serialization and deserialization.

```tsn
import { JSON } from "std:json"
```

| Method            | Signature                              | Description               |
|-------------------|----------------------------------------|---------------------------|
| `JSON.parse`      | `JSON.parse(s: str): Json`            | Parse JSON string         |
| `JSON.stringify`  | `JSON.stringify(v: unknown, space?: int): str` | Serialize to JSON |

The `Json` type represents any valid JSON value.

**Example:**

```tsn
import { JSON } from "std:json"

const raw = '{"name": "Alice", "age": 30}'
const data = JSON.parse(raw)

const output = JSON.stringify(data, 2)
print(output)
```

---

## std:math

Mathematical constants and functions.

```tsn
import { Math } from "std:math"
```

**Constants:**

| Constant   | Value        |
|------------|--------------|
| `Math.PI`  | 3.14159265…  |
| `Math.E`   | 2.71828182…  |

**Methods:**

| Method              | Signature                      | Description               |
|---------------------|--------------------------------|---------------------------|
| `Math.abs`          | `(x: float): float`            | Absolute value            |
| `Math.sqrt`         | `(x: float): float`            | Square root               |
| `Math.pow`          | `(x: float, y: float): float`  | Power                     |
| `Math.floor`        | `(x: float): int`              | Round down                |
| `Math.ceil`         | `(x: float): int`              | Round up                  |
| `Math.round`        | `(x: float): int`              | Round to nearest          |
| `Math.min`          | `(a: float, b: float): float`  | Minimum                   |
| `Math.max`          | `(a: float, b: float): float`  | Maximum                   |
| `Math.log`          | `(x: float): float`            | Natural logarithm         |
| `Math.log2`         | `(x: float): float`            | Base-2 logarithm          |
| `Math.log10`        | `(x: float): float`            | Base-10 logarithm         |
| `Math.sin`          | `(x: float): float`            | Sine (radians)            |
| `Math.cos`          | `(x: float): float`            | Cosine (radians)          |
| `Math.tan`          | `(x: float): float`            | Tangent (radians)         |
| `Math.random`       | `(): float`                    | Random float in [0, 1)    |
| `Math.trunc`        | `(x: float): int`              | Truncate to integer       |
| `Math.sign`         | `(x: float): int`              | Sign (-1, 0, 1)           |
| `Math.clamp`        | `(x: float, lo: float, hi: float): float` | Clamp to range |
| `Math.hypot`        | `(a: float, b: float): float`  | Hypotenuse                |

---

## std:path

File path manipulation.

```tsn
import { Path } from "std:path"
```

| Method             | Signature                       | Description                  |
|--------------------|---------------------------------|------------------------------|
| `Path.join`        | `(...parts: str): str`          | Join path segments           |
| `Path.normalize`   | `(p: str): str`                 | Normalize path               |
| `Path.dirname`     | `(p: str): str`                 | Parent directory             |
| `Path.basename`    | `(p: str, ext?: str): str`      | File name (with optional ext strip) |
| `Path.extname`     | `(p: str): str`                 | File extension (e.g. ".tsn") |
| `Path.isAbsolute`  | `(p: str): bool`                | Check if absolute path       |
| `Path.sep`         | `str`                           | OS path separator            |

**Example:**

```tsn
import { Path } from "std:path"

const full = Path.join("/home", "user", "file.tsn")
print(Path.dirname(full))    // /home/user
print(Path.basename(full))   // file.tsn
print(Path.extname(full))    // .tsn
```

---

## std:reflect

Runtime metadata reflection.

```tsn
import { defineMetadata, getMetadata, hasMetadata } from "std:reflect"
```

| Export           | Signature                                            | Description              |
|------------------|------------------------------------------------------|--------------------------|
| `defineMetadata` | `defineMetadata(key: str, value: unknown, target: unknown): void` | Attach metadata |
| `getMetadata`    | `getMetadata(key: str, target: unknown): unknown`    | Read metadata            |
| `hasMetadata`    | `hasMetadata(key: str, target: unknown): bool`       | Check metadata exists    |

---

## std:result

Type-safe error handling without exceptions.

```tsn
import { Result, ok, err } from "std:result"
```

| Export   | Description                          |
|----------|--------------------------------------|
| `Result` | `Result<T, E>` type (Ok or Err)      |
| `ok`     | `ok<T>(value: T): Result<T, never>`  |
| `err`    | `err<E>(error: E): Result<never, E>` |

**Result methods:**

| Method      | Description                              |
|-------------|------------------------------------------|
| `unwrap()`  | Get value or throw if Err                |
| `isOk()`    | Check if Ok variant                      |
| `isErr()`   | Check if Err variant                     |
| `map(fn)`   | Transform Ok value                       |
| `andThen(fn)` | Chain Result-returning operations      |
| `orElse(fn)` | Handle Err variant                      |

**Example:**

```tsn
import { Result, ok, err } from "std:result"

function divide(a: int, b: int): Result<float, str> {
    if b === 0 {
        return err("division by zero")
    }
    return ok(a / b)
}

const r = divide(10, 2)
if r.isOk() {
    print(r.unwrap())  // 5
}
```

---

## std:sys

System information and environment.

```tsn
import { Sys } from "std:sys"
```

| Method          | Signature               | Description                    |
|-----------------|-------------------------|--------------------------------|
| `Sys.platform`  | `(): str`               | OS name ("linux", "windows", "macos") |
| `Sys.cwd`       | `(): str`               | Current working directory      |
| `Sys.args`      | `(): str[]`             | Command-line arguments         |
| `Sys.env`       | `(key: str): str\|null` | Environment variable           |
| `Sys.exit`      | `(code: int): never`    | Exit process                   |

---

## std:test

Assertions for testing.

```tsn
import { assert, assertEqual, assertNotEqual, fail, ok } from "std:test"
```

| Export          | Signature                                      | Description              |
|-----------------|------------------------------------------------|--------------------------|
| `assert`        | `assert(cond: bool, msg?: str): void`          | Assert condition is true |
| `assertEqual`   | `assertEqual(a: unknown, b: unknown, msg?: str): void` | Assert equality  |
| `assertNotEqual`| `assertNotEqual(a: unknown, b: unknown): void` | Assert inequality        |
| `fail`          | `fail(msg: str): never`                        | Fail unconditionally     |
| `ok`            | `ok(v: unknown, msg?: str): void`              | Assert truthy            |

---

## std:time

Time and date utilities.

```tsn
import { Time } from "std:time"
```

| Method              | Signature      | Description                         |
|---------------------|----------------|-------------------------------------|
| `Time.now`          | `(): int`      | Unix timestamp in seconds           |
| `Time.millis`       | `(): int`      | Unix timestamp in milliseconds      |
| `Time.toISOString`  | `(ts: int): str` | Format timestamp as ISO 8601 string |

---

## Builtins

These are available without any import.

### Functions

| Name      | Signature                        | Description                    |
|-----------|----------------------------------|--------------------------------|
| `print`   | `print(...args: unknown): void`  | Print to stdout with newline   |
| `assert`  | `assert(cond: bool, msg: str): void` | Runtime assertion          |

### Constants

| Name       | Type    | Value                  |
|------------|---------|------------------------|
| `Infinity` | `float` | Positive infinity      |
| `NaN`      | `float` | Not a Number           |

### Primitive Type Methods

All primitive types have built-in methods accessible via dot syntax.

**str:**
```tsn
"hello".length           // 5
"hello".toUpperCase()    // "HELLO"
"hello".includes("ell")  // true
"hello".split("l")       // ["he", "", "o"]
"  hi  ".trim()          // "hi"
"abc".reverse()          // "cba"
"hi".repeat(3)           // "hihihi"
```

**int:**
```tsn
42.toString()            // "42"
42.toFloat()             // 42.0
(-5).abs()               // 5
42.toHex()               // "2a"
int.MAX_VALUE            // 9223372036854775807
int.MIN_VALUE            // -9223372036854775808
```

**float:**
```tsn
3.14.floor()             // 3
3.14.ceil()              // 4
3.14.round()             // 3
3.14.abs()               // 3.14
3.14.toInt()             // 3
```

**Array\<T\>:**
```tsn
[1,2,3].length           // 3
[1,2,3].push(4)          // [1,2,3,4]
[1,2,3].pop()            // 3
[1,2,3].map(x => x * 2) // [2,4,6]
[1,2,3].filter(x => x > 1) // [2,3]
[1,2,3].find(x => x > 1)   // 2
[1,2,3].includes(2)      // true
[1,2,3].slice(1, 2)      // [2]
[1,2,3].reverse()        // [3,2,1]
[1,2,3].join(", ")       // "1, 2, 3"
```

**bool:**
```tsn
true.toString()          // "true"
```
