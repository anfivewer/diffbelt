Diffbelt
========

Immutable key-value database with main focus on taking diffs belween versions of data (we call them **generations**) and to be able to transform one set of key-value items (**collection**) to another set and preserve consistency even in cases of system failures.

# Basics

## Entities

* **Collection** — lexicographically ordered set of key-value pairs
  * **CollectionName** — `1` to `255` bytes that are represent UTF-8 string
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
  * **ReaderName** — non-empty UTF-8 string (TODO: add limit, issue [#10](https://github.com/anfivewer/diffbelt/issues/10))
  * **CollectionName** — optional **CollectionName**, if not specified — it means current collection
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

type EncodedString = {
    value: string;
    encoding?: Encoding;
};

type KeyValue = {
    key: EncodedString;
    value: EncodedString;
};

type KeyValueUpdate = {
    key: EncodedString;
    ifNotPresent?: boolean;
    value: EncodedString | null;
};
```

## `GET /collections/`

```
type Response = {
    items: {
        name: string;
        isManual: boolean;
    }[];
};
```

Returns list of all collections.

## `POST /collections/`

```
type Request = {
    collectionName: string;
    encoding?: Encoding;
} &
(
    {
        isManual: false
    }
  | {
        isManual: true,
        initialGenerationId: EncodedString;
    }
);

type Response = {
    generationId: EncodedString;
};
```

Creates collection. For manual collections you can specify `initialGenerationId: ""` (empty string).

## `GET /collections/:collectionName`

```
type QueryParams = {
    // as string, separated by comma
    fields?: ('generationId' | 'nextGenerationId')[],
};

type Response = {
    isManual: boolean;
    generationId?: EncodedString;
    nextGenerationId?: EncodedString;
};
```

By default, all fields are returned, but you can specify only needed:

```
GET /collections/log-lines?fields=generationId,nextGenerationId

{
    "isManual": false,
    "generationId": {"value": "AAAAAAAACm4=", "encoding": "base64"},
    "nextGenerationId": {"value": "AAAAAAAACm8=", "encoding": "base64"}
}
```

## `DELETE /collections/:collectionName`

Deletes the collection. Warning: this will delete it with all files immediately. In the future I plan to just move it and delete in a week or something like that to be able to recover it if it was unattended action.

Deletion of associated readers is not implemented yet. Issue [#2](https://github.com/anfivewer/diffbelt/issues/2).

## `GET /collections/:collectionName/generationId/stream`

```
type QueryParams = {
    generationId?: string;
    generationIdEncoding?: Encoding;
};

type Response = {
    generationId: EncodedString;
};
```

If `generationId` query param is not set, responds immediately with collection current `generationId`. If param is set, responds with updated `generationId` or with the same one if 60 seconds passed.

This long-polling is useful to wait for commits of non-manual collection, or wait for changes to run diff on some collection.

## `POST /collections/:collectionName/get`

```
type Request = {
    key: EncodedString;
    generationId?: EncodedString;
    phantomId?: EncodedString;
};

type Response = {
    generationId: EncodedString;
    item: KeyValue | null;
};
```

## `POST /collections/:collectionName/getKeysAround`

```
type Request = {
    key: EncodedString;
    requireKeyExistance: boolean;
    generationId?: EncodedString;
    phantomId?: EncodedString;
};

type Response = {
    generationId: EncodedString,
    left: EncodedString[];
    right: EncodedString[];
    hasMoreOnTheLeft: boolean;
    hasMoreOnTheRight: boolean;
    foundKey: boolean;
};
```

Beware, `Response['left']` is in reversed keys order. For example, if you are requesting keys around `4`, `left` will contain `[{"key": "3"}, {"key": "2"}, {"key": "1"}]`.

`requireKeyExistance: false` case is not implemented yet. Issue [#3](https://github.com/anfivewer/diffbelt/issues/3).

## `POST /collections/:collectionName/put`

```
type Request = {
    item: KeyValueUpdate;
    generationId?: EncodedString;
    phantomId?: EncodedString;
};

type Response = {
    generationId: EncodedString;
    wasPut?: boolean;
};
```

Writes single key-value entry (or deletes it if `value: null`). For manual collections `generationId` is required and must be equal to started generation (except if `phantomId` used, then `generationId` can have any value in the past or in the future).

If `ifNotPresent: true`, then if `key` already exists, its value will not be overwritten and `generationId` of this `key` will not be updated. `wasPut` will indicate, was value updated or not.

Warning: without `ifNotPresent` key-value record will be updated even if it has the same value. For example if you have `{"key":"a", "value":"42", "generationId":"001"}` stored in the database and next `generationId` is `002`, if you'll `/put` `{"key":"a", "value":"42"}`, new record `{"key":"a", "value":"42", "generationId":"002"}` will be created. Vote for issue [#1](https://github.com/anfivewer/diffbelt/issues/1).

## `POST /collections/:collectionName/putMany`

```
type Request = {
    items: KeyValueUpdate[];
    generationId?: EncodedString;
    phantomId?: EncodedString;
};

type Response = {
    generationId: EncodedString;
};
```

## `GET /collections/:collectionName/readers/`

```
type Response = {
    items: {
        readerName: string;
        collectionName?: string;
        generationId: EncodedString;
    }[];
};
```

## `POST /collections/:collectionName/readers/`

```
type Request = {
    readerName: string;
    collectionName?: string;
    generationId?: EncodedString | null;
};

type Response = {};
```

## `DELETE /collections/:collectionName/readers/:readerName`

```
type Response = {};
```

## `PUT /collections/:collectionName/readers/:readerName`

```
type Request = {
    generationId?: EncodedString | null;
};

type Response = {};
```

## `POST /collections/:collectionName/diff/start`

Request parameters are broken, see issue [#5](https://github.com/anfivewer/diffbelt/issues/5).

```
type Request = {
    toGenerationId?: EncodedString;
} & (
    {
        fromGenerationId: EncodedString;
    }
  | {
        fromReader: {
            readerName: string;
            collectionName?: string;
        };
    }    
);

type KeyValueDiff = {
    key: EncodedString;
    fromValue: EncodedString | null,
    intermediateValues: (EncodedString | null)[];
    toValue: EncodedString | null,
};

type DiffResponse = {
    fromGenerationId: EncodedString,
    toGenerationId: EncodedString;
    items: KeyValueDiff[];
    cursorId?: string;
};

type Response = DiffResponse
```

There is two ways to specify `fromGenerationId`:

* Manually by providing `fromGenerationId`
* By providing `fromReader`. If specified, diff will read `readerName` from collection `collectionName`, take its `generationId`

Response can have `generationId` that is less or equal to `toGenerationId` (if it is specified, or to current `generationId`). You should repeat diff requests until it will respond with `fromGenerationId == generationId`.

`intermediateValues` currently always is an empty array. Later there will be `omitIntermediateValues: false` option that will provide those values. See issue [#6](https://github.com/anfivewer/diffbelt/issues/6).

## `POST /collections/:collectionName/diff/next`

```
type Request = {
    cursorId: string;
};

type Response = DiffResponse;
```

If `diff/start` responded with `cursorId` you should call this method to get the rest of output.

## `POST /collections/:collectionName/diff/abort`

Not implemented yet. Issue [#7](https://github.com/anfivewer/diffbelt/issues/7).

## `POST /collections/:collectionName/query/start`

```
type Request = {
    generationId?: EncodedString;
    phantomId?: EncodedString;
};

type QueryResponse = {
    generationId: EncodedString;
    items: KeyValue[];
    cursorId?: string;
};

type Response = QueryResponse
```

Reads all key-value records from collection. If `generationId` is specified, items that was added/updated/deleted after this generation will be omitted from the result.

## `POST /collections/:collectionName/query/next`

```
type Request = {
    cursorId: string;
};

type Response = DiffResponse;
```

If `query/start` responded with `cursorId` you should call this method to get the rest of output.

## `POST /collections/:collectionName/query/abort`

Not implemented yet. Issue [#7](https://github.com/anfivewer/diffbelt/issues/7).

## `POST /collections/:collectionName/phantom/start`

```
type Request = {};

type Response = {
    phantomId: EncodedString;
};
```

Gets `phantomId` that can be used for puts. They are useful to create "fake modifications" of some collection in the past. Records with `phantomId` is visible only for query/getKeysAround with specified `phantomId` (and only for equal `phantomId`).

Phantoms are relatively short-living entity. Currently, their TTL is not specified, but in next revisions I maybe will remove this method and will bind phantoms to generations (when you start generation you can create phantoms in some collections, then after commit phantoms are gone).

## `POST /collections/:collectionName/generation/start`

```
type Request = {
    generationId: EncodedString;
    abortOutdated?: boolean;
};

type Response = {};
```

Works only on manual collections.

If `abortOutdated` specified and there is generation that is already started and its `generationId` is less than provided, all records that was added in this generation will be deleted.

## `POST /collections/:collectionName/generation/abort`

```
type Request = {
    generationId: EncodedString;
};

type Response = {};
```

Aborts generation, deletes all records that was put in this generation.

## `POST /collections/:collectionName/generation/commit`

```
type Request = {
    generationId: EncodedString;
    updateReaders?: {
        readerName: string;
        generationId: EncodedString;
    }[];
};

type Response = {};
```

Commits generation (makes new records visible), atomically with readers updates.

<a name="transformExample"></a>For example, you need to transform collections `A` and `B` to collection `C`. Initialization:

* Create manual collection `C` with `generationId: {value: "AAAAAAAAAAA=", "encoding": "base64"}` (64 zero bits)
* Create reader in collection `C`: `{"readerName": "from_a", "collectionName": "A", "generationId": {value:""}}`
* Create reader in collection `C`: `{"readerName": "from_b", "collectionName": "B", "generationId": {value:""}}`

Transform iteration:

* Get current&next `C` generation ids
* Get next generationId if it is present, if not — take current
* Increment it (from `AAAAAAAAAAA=` it will become `AAAAAAAAAAE=`, then `AAAAAAAAAAI=` and so on), start generation with incremented `generationId` and `abortOutdated: true`, we'll call this generation id as `commitGenerationId`
* Execute diff on collection `A` with `readerName: 'from_a', readerCollectionName: 'C'`, remember `generationId` of diff result as `aGenerationId`
* Execute diff on collection `B` with `readerName: 'from_b', readerCollectionName: 'C'`, remember `generationId` of diff result as `bGenerationId`
* Process diff, make puts to collection `C` (`generationId` should be `commitGenerationId`); you can also make gets with `commitGenerationId` to see what you are already stored to some key to update it, if you got new data from `A` or `B`
* Commit generation `commitGenerationId`, pass:
  ```
  updateReaders: [
      { readerName: 'from_a', generationId: aGenerationId },
      { readerName: 'from_b', generationId: bGenerationId },
  ]
  ```

If you got any error on steps above — abort generation and try again/investigate your code.

Repeat transform iteration until readers `from_a` and `from_b` will not be equal to `A` and `B` generation ids correspondingly.  Then you can watch for `A` and `B` generation ids, wait for their updates and repeat the process.