* Write to `phantoms` column family when putting a phantom
* Continue to use `SingleGeneration` mode if it was selected, not gegradate to
  `InMemory` after cursor continuation
* Collect system status stats, like cleanup progress, not finished generations, etc
* Investigate concurrent writes to `COLLECTION_CF_META`, maybe we need to read after write to ensure that our operation is not collided with other one 

-----

# Misc

Regexp for purging dev printlines:

```
[^e]println!\((?!"cargo|"Temp)
```