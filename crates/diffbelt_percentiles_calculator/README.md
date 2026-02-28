diffbelt-percentiles-calculator
-------------------------------

Provides utilities to perform streaming percentiles calculation. Streaming
percentiles calculation can be implemented with:

- Collection of initial data, no need to be ordered
- Intermediate collection of sorted data (mapped from initial, grouped by period
  and ordered by percentiles key)
- Collection of resulting percentiles of period buckets from intermediate collection

Database which stores this collections should:

- Be able to get diff (which records are added, removed, updated and how) from
  time when we seen collections last time
- Be able to get keys around specified key for past state of data
- Be able to temporarily mutate those keys in the past (add/remove keys)

Algorithm of recalculation of intermediate collection to final collection:

- Receive chunk of the diff, group records by period
- For each period fetch target record
- For each target record see which percentiles needs to be calculated, fetch
  keys around those percentiles
- Go for each change in the diff.
  - Modification of key is equal to its removal, then addition
  - When key is added/removed — move pointers of current percentiles in theirs
    "keys around"
  - If pointer is close to end of fetched keys - request fetching next keys
  - Mutate "keys around" with changed keys (do request to database to respond
    with changed "keys around" in the future) AND store all previous keys
    changes which are outside of currenlty loaded "keys around"
    (you can not store keys if there is no pending fetches to "keys around")
  - If saved keys outside of "keys around" reaches limit — wait for all "keys
    around" fetches
  - When you are receiving "keys around" — see for key changes and apply them to
    what you're received and remove them from stored keys.
- When there is no more changes — save percentiles
