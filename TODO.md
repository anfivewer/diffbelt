* Continue to use `SingleGeneration` mode if it was selected, not gegradate to
  `InMemory` after cursor continuation
* Collect system status stats, like cleanup progress, not finished generations, etc
* Investigate concurrent writes to `COLLECTION_CF_META`, maybe we need to read after write to ensure that our operation is not collided with other one 
* Write tests for generation bound phantoms, check that they gc after query/diff (when there will be raw db api)
* Disallow parsing Flatbuffers from random byte slices, do it only from 8-bytes padded slices to be able to shift data to ensure alignment
* Limit `gc_iterator` lookups count (writing too much phantoms may crash server on gc collection of this phantoms)
* Move `items_limit` out of DiffLogic, it miscalculates on equal values
* Ensure some alignment for input slices going to wasm (and/or make input mutable?)
* Remove Regex wasm bindings
* Do not forget to call deallocs which are scheduled
* Return diffbelt parseable flatbuffers errors as it
* Move `if_not_present` out of `KeyValueUpdate` for put_many
* Tests for empty aggregate transform
