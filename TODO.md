* Write to `phantoms` column family when putting a phantom
* Continue to use `SingleGeneration` mode if it was selected, not gegradate to
  `InMemory` after cursor continuation

-----

# Misc

Regexp for purging dev printlines:

```
[^e]println!\("(?!cargo|Temp)
```