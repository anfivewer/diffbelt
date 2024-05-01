* Write to `phantoms` column family when putting a phantom
* Continue to use `SingleGeneration` mode if it was selected, not gegradate to
  `InMemory` after cursor continuation
* Collect system status stats, like cleanup progress, not finished generations, etc

-----

# Misc

Regexp for purging dev printlines:

```
[^e]println!\((?!"cargo|"Temp)
```