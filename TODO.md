* Continue to use `SingleGeneration` mode if it was selected, not gegradate to
  `InMemory` after cursor continuation
* Collect system status stats, like cleanup progress, not finished generations, etc
* Investigate concurrent writes to `COLLECTION_CF_META`, maybe we need to read after write to ensure that our operation is not collided with other one 
* Write tests for generation bound phantoms, check that they gc after query/diff (when there will be raw db api)

-----

# Misc

Regexp for purging dev printlines:

```
[^e]println!\((?!"cargo|"Temp)
```