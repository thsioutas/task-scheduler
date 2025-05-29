# Some design choices are explained here


## TaskStorage

### Original Considerations
The `TaskStorage` component used by `TaskScheduler` had three initial design options:

1. Pass `TaskStorage` as a function argument
2. Dynamic dispatch using `Box<dyn TaskStorage>`
3. Static dispatch using generics: `TaskScheduler<T>`

**Option 1** (passing as argument) was rejected due to:
* Increased number of function parameters
* Poor encapsulation of state and logic

**Option 2** (dynamic dispatch via `Box<dyn TaskStorage>`) was considered but ultimately not chosen because:
* Dynamic dispatch is unnecessary since the backend is only chosen once at startup
* Requires heap allocation and loses some compile-time guarantees

**Option 3** (generics) was initially preferred for:
* Compile-time type safety
* Better runtime performance (no vtables or heap allocation)
* Cleaner ownership semantics in TaskScheduler

### Final Approach: Enum-Based Storage Wrapper
Instead of relying on trait objects or generics, the final implementation uses an **enum wrapper** (`TaskStorageBackend`)
that internally matches and delegates to specific implementations like `InMemoryStorage` or `PostgresStorage`.

This approach offers a balanced tradeoff:
* Avoids dynamic dispatch and heap allocation
* Eliminates the need for generics and monomorphization overhead
* Keeps type-switching logic localized to one enum
* Maintains clean, testable API with a common interface