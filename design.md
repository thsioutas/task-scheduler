# Some design choices are explained here


## TaskStorage
For how the `TaskStorage` will be used by the `TaskScheduler` the following options are considered:
1. Pass `TaskStorage` as an argument
2. Dynamic dispatch (`Box<dyn TaskStorage>`)
3. Static dispatch (`TaskScheduler<T>`)

First option is rejected:
* More function arguments
* Less encapsulation

Since the storage type is configured only once in main at startup, there's no real need for dynamic dispatch (Box<dyn TaskStorage>). Using generics (TaskScheduler<T>) makes more sense because:
* Better performance – No dynamic dispatch overhead.
* No heap allocation – Everything is stored directly on the stack.
* Compile-time guarantees – Rust ensures everything is correct at compile time.
* Cleaner code – TaskScheduler owns the storage, making state management simpler.