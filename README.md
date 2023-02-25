Diffbelt
========

Immutable key-value database with main focus on taking diffs belween versions of data (we call them **generations**) and to be able to transform one set of key-value items (**collection**) to another set and preserve consistency even in cases of system failures.

# Basics

## Entities

* **Collection** — lexicographically ordered set of key-value pairs
  * **CollectionId** — `1` to `255` bytes that are represent UTF-8 string
  * **CurrentGenerationId** — zero to `255` bytes
  * **NextGenerationId** — `NULL` or `1` to `255` bytes, must be bigger than **CurrentGenerationId**. Can be `NULL` in manual collections where no next generation is planned yet.
  * **Manual collection** — collection allows puts only with specified `generationId` that previosly was initiated with `generation/start`
  * **Non-manual collection** — collection that allows puts without `generationId` and automatically commits **NextGenerationId** to **CurrentGenerationId** after some amount of puts/time elapsed
* **Record** — item of **collection**, consists of:
  * **Key** — zero to `2^24-1` bytes
  * **Value** — `TOMBSTONE` or zero to `out of memory exception` bytes (but in fact this is limited by http methods request body size, for example there is 32 megabytes for `putMany`). `TOMBSTONE` means that record with such `key` was deleted
  * **GenerationId** — zero to `255` bytes. Any query always uses some `generationId` (latest for the collection or provided by user), if record's `generationId` is bigger than `generationId` for the query's, this record is invisible. Only one **record** with the maximally close `generationId` to the query's `generationId` is visible
  * **PhantomId** — `1` to `255` bytes. Records with `phantomId` are visible only for queries with the same `phantomId`
* **Reader** — item of **collection**, consists of:
  * **ReaderId** — non-empty UTF-8 string (TODO: add limit, issue [#10](https://github.com/anfivewer/diffbelt/issues/10))
  * **CollectionName** — optional **CollectionId**, if not specified — it means current collection
  * **GenerationId** — **CurrentGenerationId**, marker to some generation in foreign collection (specified by **CollectionName**). It prevents garbage collection of generations of target collection and may be used for `diff` calls as `fromGenerationId` source

## Transform flow examples

TODO.

Currently, you can see [this example](#transformExample), or you can try to dive into https://github.com/anfivewer/an5wer/blob/d680fc113447bbf2c03b6ea050769b2ffcab9b5c/packages-sesuritu/logs-processing/src/main.ts#L63 .

# API

This is version zero (or maybe even `-1`), it will change dramatically, since it is very inconsistent/encodings was added after first planning, some methods are plain POST requests because it was easier to implement in the start of my first touches of `hyper` http lib.

## Base types

Input/output parameters types are described in TypeScript-like type definitions.

* `type Request` is definition of JSON of the body of a request
* `type Response` is definition of JSON of the body of a response
* `type QueryParams` is a set of params that are can be specified as query params (with comment of how it maps from a string)
* Rest types are helpers and can be reused between methods

```ts
// default value is 'utf8'
type Encoding = 'utf8' | 'base64';

type KeyValue = {
    key: string;
    keyEncoding?: Encoding;
    value: string;
    valueEncoding?: Encoding;
};

type KeyValueUpdate = {
    key: string;
    keyEncoding?: Encoding;
    ifNotPresent?: boolean;
    value: string | null;
    valueEncoding?: Encoding;
};

type EncodedKey = {
    key: string;
    keyEncoding?: Encoding;
};

type EncodedValue = {
    value: string;
    encoding?: Encoding;
};
```

## `GET /collections/`

```
type Response = {
    items: {
        name: string;
        isManual: boolean;
        generationId: string;
        generationIdEncoding?: Encoding;
    }[];
};
```

Returns list of all collections.

## `POST /collections/`

```
type Request = {
    collectionId: string;
    encoding?: Encoding;
} &
(
    {
        isManual: false
    }
  | {
        isManual: true,
        initialGenerationId: string;
        initialGenerationIdEncoding?: Encoding;
    }
);

type Response = {
    generationId: string;
    generationIdEncoding?: Encoding;
};
```

Creates collection. For manual collections you can specify `initialGenerationId: ""` (empty string).

## `GET /collections/:collectionId`

```
type QueryParams = {
    // as string, separated by comma
    fields?: ('generationId' | 'nextGenerationId')[],
};

type Response = {
    isManual: boolean;
    generationId?: string;
    generationIdEncoding?: Encoding;
    nextGenerationId?: string;
    nextGenerationIdEncoding?: Encoding;
};
```

By default, all fields are returned, but you can specify only needed:

```
GET /collections/log-lines?fields=generationId,nextGenerationId

{
    "isManual": false,
    "generationId": "AAAAAAAACm4=",
    "generationIdEncoding": "base64",
    "nextGenerationId": "AAAAAAAACm8=",
    "nextGenerationIdEncoding": "base64"
}
```

## `DELETE /collections/:collectionId`

Deletes the collection. Warning: this will delete it with all files immediately. In the future I plan to just move it and delete in a week or something like that to be able to recover it if it was unattended action.

Deletion of associated readers is not implemented yet. Issue [#2](https://github.com/anfivewer/diffbelt/issues/2).

## `GET /collections/:collectionId/generationId/stream`

```
type QueryParams = {
    generationId?: string;
    generationIdEncoding?: Encoding;
};

type Response = {
    generationId: string;
    generationIdEncoding?: Encoding;
};
```

If `generationId` query param is not set, responds immediately with collection current `generationId`. If param is set, responds with updated `generationId` or with the same one if 60 seconds passed.

This long-polling is useful to wait for commits of non-manual collection, or wait for changes to run diff on some collection.

## `POST /get`

```
type Request = {
    collectionId: string;
    key: string;
    keyEncoding?: Encoding;
    generationId?: string;
    generationIdEncoding?: Encoding;
    phantomId?: string;
    phantomIdEncoding?: Encoding;
    encoding?: Encoding;
};

type Response = {
    generationId: string;
    generationIdEncoding?: Encoding;
    item: KeyValue | null;
};
```

## `POST /getKeysAround`

```
type Request = {
    collectionId: string;
    key: string;
    keyEncoding?: Encoding;
    requireKeyExistance: boolean;
    generationId?: string;
    generationIdEncoding?: Encoding;
    phantomId?: string;
    phantomIdEncoding?: Encoding;
    encoding?: Encoding,
};

type Response = {
    generationId: string,
    generationIdEncoding?: Encoding,
    left: EncodedKey[];
    right: EncodedKey[];
    hasMoreOnTheLeft: boolean;
    hasMoreOnTheRight: boolean;
    foundKey: boolean;
};
```

Beware, `Response['left']` is in reversed keys order. For example, if you are requesting keys around `4`, `left` will contain `[{"key": "3"}, {"key": "2"}, {"key": "1"}]`.

`requireKeyExistance: false` case is not implemented yet. Issue [#3](https://github.com/anfivewer/diffbelt/issues/3).

## `/getMany`

Not implemented yet.

## `POST /put`

```
type Request = {
    collectionId: string;
    key: string;
    keyEncoding?: Encoding;
    ifNotPresent?: boolean;
    value: string | null;
    valueEncoding?: Encoding;
    generationId?: string;
    generationIdEncoding?: Encoding;
    phantomId?: string;
    phantomIdEncoding?: Encoding;
    encoding?: Encoding;
};

type Response = {
    generationId: string;
    generationIdEncoding?: Encoding;
    wasPut?: boolean;
};
```

Writes single key-value entry (or deletes it if `value: null`). For manual collections `generationId` is required and must be equal to started generation (except if `phantomId` used, then `generationId` can have any value in the past or in the future).

If `ifNotPresent: true`, then if `key` already exists, its value will not be overwritten and `generationId` of this `key` will not be updated. `wasPut` will indicate, was value updated or not.

Warning: without `ifNotPresent` key-value record will be updated even if it has the same value. For example if you have `{"key":"a", "value":"42", "generationId":"001"}` stored in the database and next `generationId` is `002`, if you'll `/put` `{"key":"a", "value":"42"}`, new record `{"key":"a", "value":"42", "generationId":"002"}` will be created. Vote for issue [#1](https://github.com/anfivewer/diffbelt/issues/1).

## `POST /collections/:collectionId/putMany`

```
type Request = {
    items: KeyValueUpdate[];
    generationId?: string;
    generationIdEncoding?: Encoding;
    phantomId?: string;
    phantomIdEncoding?: Encoding;
    encoding?: Encoding;
};

type Response = {
    generationId: string;
    generationIdEncoding?: Encoding;
};
```

## `POST /collections/:collectionId/reader/list`

```
type Request = {};

type Response = {
    items: {
        readerId: string;
        collectionName?: string;
        generationId: string;
        generationIdEncoding?: Encoding;
    }[];
};
```

## `POST /collections/:collectionId/reader/create`

```
type Request = {
    readerId: string;
    collectionName?: string;
    generationId?: string | null;
    generationIdEncoding?: Encoding;
};

type Response = {};
```

## `POST /collections/:collectionId/reader/delete`

```
type Request = {
    readerId: string;
};

type Response = {};
```

## `POST /collections/:collectionId/reader/update`

```
type Request = {
    readerId: string;
    generationId?: string | null;
    generationIdEncoding?: Encoding;
};

type Response = {};
```

## `POST /collections/:collectionId/diff/start`

Request parameters are broken, see issue [#5](https://github.com/anfivewer/diffbelt/issues/5).

```
type Request = {
    // FIXME
    readerId?: string;
    readerCollectionName?: string;
};

type KeyValueDiff = {
    key: string;
    keyEncoding?: Encoding;
    fromValue: EncodedValue | null,
    intermediateValues: (EncodedValue | null)[];
    toValue: EncodedValue | null,
};

type DiffResponse = {
    fromGenerationId: {
        value: string;
        encoding?: Encoding;
    },
    generationId: string;
    generationIdEncoding?: Encoding;
    items: KeyValueDiff[];
    cursorId?: string;
};

type Response = DiffResponse
```

There is two ways to specify `fromGenerationId`:

* Manually by providing `fromGenerationId`
* By providing `readerId` and `readerCollectionName`. If specified, diff will read `readerId` from collection `readerCollectionName`, take its `generationId`

Response can have `generationId` that is less or equal to `toGenerationId` (if it is specified, or to current `generationId`). You should repeat diff requests until it will respond with `fromGenerationId == generationId`.

`intermediateValues` currently always is an empty array. Later there will be `omitIntermediateValues: false` option that will provide those values. See issue [#6](https://github.com/anfivewer/diffbelt/issues/6).

## `POST /collections/:collectionId/diff/next`

```
type Request = {
    cursorId: string;
};

type Response = DiffResponse;
```

If `diff/start` responded with `cursorId` you should call this method to get the rest of output.

## `POST /collections/:collectionId/diff/abort`

Not implemented yet. Issue [#7](https://github.com/anfivewer/diffbelt/issues/7).

## `POST /collections/:collectionId/query/start`

```
type Request = {
    generationId?: string;
    generationIdEncoding?: Encoding;
    phantomId?: string;
    phantomIdEncoding?: Encoding;
};

type QueryResponse = {
    generationId: string;
    generationIdEncoding?: Encoding;
    items: KeyValue[];
    cursorId?: string;
};

type Response = QueryResponse
```

Reads all key-value records from collection. If `generationId` is specified, items that was added/updated/deleted after this generation will be omitted from the result.

## `POST /collections/:collectionId/query/next`

```
type Request = {
    cursorId: string;
};

type Response = DiffResponse;
```

If `query/start` responded with `cursorId` you should call this method to get the rest of output.

## `POST /collections/:collectionId/query/abort`

Not implemented yet. Issue [#7](https://github.com/anfivewer/diffbelt/issues/7).

## `POST /collections/:collectionId/phantom/start`

```
type Request = {};

type Response = {
    phantomId: string;
    phantomIdEncoding?: Encoding;
};
```

Gets `phantomId` that can be used for puts. They are useful to create "fake modifications" of some collection in the past. Records with `phantomId` is visible only for query/getKeysAround with specified `phantomId` (and only for equal `phantomId`).

Phantoms are relatively short-living entity. Currently, their TTL is not specified, but in next revisions I maybe will remove this method and will bind phantoms to generations (when you start generation you can create phantoms in some collections, then after commit phantoms are gone).

## `POST /collections/:collectionId/generation/start`

```
type Request = {
    generationId: string;
    generationIdEncoding?: Encoding;
    abortOutdated?: boolean;
};

type Response = {};
```

Works only on manual collections.

If `abortOutdated` specified and there is generation that is already started and its `generationId` is less than provided, all records that was added in this generation will be deleted.

## `POST /collections/:collectionId/generation/abort`

```
type Request = {
    generationId: string;
    generationIdEncoding?: Encoding;
};

type Response = {};
```

Aborts generation, deletes all records that was put in this generation.

## `POST /collections/:collectionId/generation/commit`

```
type Request = {
    generationId: string;
    generationIdEncoding?: Encoding;
    updateReaders?: {
        readerId: string;
        generationId: string;
        generationIdEncoding?: Encoding;
    }[];
};

type Response = {};
```

Commits generation (makes new records visible), atomically with readers updates.

<a name="transformExample"></a>For example, you need to transform collections `A` and `B` to collection `C`. Initialization:

* Create manual collection `C` with `generationId: "AAAAAAAAAAA=", "generationIdEncoding": "base64"` (64 zero bits)
* Create reader in collection `C`: `{"readerId": "from_a", "collectionName": "A", "generationId": ""}`
* Create reader in collection `C`: `{"readerId": "from_b", "collectionName": "B", "generationId": ""}`

Transform iteration:

* Get current&next `C` generation ids
* Get next generationId if it is present, if not — take current
* Increment it (from `AAAAAAAAAAA=` it will become `AAAAAAAAAAE=`, then `AAAAAAAAAAI=` and so on), start generation with incremented `generationId` and `abortOutdated: true`, we'll call this generation id as `commitGenerationId`
* Execute diff on collection `A` with `readerId: 'from_a', readerCollectionName: 'C'`, remember `generationId` of diff result as `aGenerationId`
* Execute diff on collection `B` with `readerId: 'from_b', readerCollectionName: 'C'`, remember `generationId` of diff result as `bGenerationId`
* Process diff, make puts to collection `C` (`generationId` should be `commitGenerationId`); you can also make gets with `commitGenerationId` to see what you are already stored to some key to update it, if you got new data from `A` or `B`
* Commit generation `commitGenerationId`, pass:
  ```
  updateReaders: [
      { readerId: 'from_a', generationId: aGenerationId },
      { readerId: 'from_b', generationId: bGenerationId },
  ]
  ```

If you got any error on steps above — abort generation and try again/investigate your code.

Repeat transform iteration until readers `from_a` and `from_b` will not be equal to `A` and `B` generation ids correspondingly.  Then you can watch for `A` and `B` generation ids, wait for their updates and repeat the process.