# [0.6.0](https://github.com/robgonnella/r-lanscan/releases/tag/v0.6.0) - 2026-01-31

### 🚀 Features

- _(term)_ enables text selection and copy in TUI [_(5f51f98)_](https://github.com/robgonnella/r-lanscan/commit/5f51f98dc1e123677fd76a2a6a4ada453c312549)

### 🐛 Bug Fixes

- _(term)_ fixes issue in device view [_(5cc5793)_](https://github.com/robgonnella/r-lanscan/commit/5cc5793fc669852599145a60eceb190201639a45)

- fix devices legend [_(9d264ef)_](https://github.com/robgonnella/r-lanscan/commit/9d264ef6d05591e44714e74175c4617d54b0cb2f)

### 🚜 Refactor

- _(term)_ refactors rendering in tui [_(15ab84b)_](https://github.com/robgonnella/r-lanscan/commit/15ab84b61893ba5bd5093d5e39d4b505a2d8a9cb)

### ⚡ Performance

- improves renderer performance [_(d2e0e0a)_](https://github.com/robgonnella/r-lanscan/commit/d2e0e0a74336e007b562da23122911fc21f1b664)

# [0.5.0](https://github.com/robgonnella/r-lanscan/releases/tag/v0.5.0) - 2026-01-26

### 🚀 Features

- _(term)_ enable mouse scrolling for devices view [_(fcdbd13)_](https://github.com/robgonnella/r-lanscan/commit/fcdbd134ea5bcff9ad30e8de6ceb2a2a867a193f)

- _(term)_ adds initial view for logs in r-lanterm [_(00a18f5)_](https://github.com/robgonnella/r-lanscan/commit/00a18f5e704e572e960dde7c57bb9cb447a44139)

- implements builder pattern for scanners [_(6102da2)_](https://github.com/robgonnella/r-lanscan/commit/6102da2bfffd88348f333a2783989098fff095a2)

### 🐛 Bug Fixes

- improves UI header and regenerates snapshots [_(d00023a)_](https://github.com/robgonnella/r-lanscan/commit/d00023ac23befbb62ad45a2e8ad78893e5884243)

- _(term)_ improves focus logic while editing [_(4f934d0)_](https://github.com/robgonnella/r-lanscan/commit/4f934d0024292b92d5ba22b178fe2b44b5ed9a2d)

- uses the more cannonical (tx, rx) order for wire (sender, reader) [_(5bacaa8)_](https://github.com/robgonnella/r-lanscan/commit/5bacaa8b2d4b312d42fdf674051c950f22884672)

- _(term)_ fixes small bug in tui [_(ed8ea17)_](https://github.com/robgonnella/r-lanscan/commit/ed8ea173c9f821fd3b30749596751757f900e028)

### 🚜 Refactor

- _(lib)_ minor refactor to processing incoming packets [_(210671f)_](https://github.com/robgonnella/r-lanscan/commit/210671ff17cf1c98de8c8417ec698400d49cc20b)

- uses "self.clone" to access scanner props in threads [_(3491ab4)_](https://github.com/robgonnella/r-lanscan/commit/3491ab410495959945e4bd5155dbcb5b9d1571c1)

- _(lib)_ improves setup and handling of heartbeats [_(03546fe)_](https://github.com/robgonnella/r-lanscan/commit/03546fecece01a23ef32606edc25af304cb34bf0)

- _(term)_ minor cleanup in term/main.rs [_(5f8b3e0)_](https://github.com/robgonnella/r-lanscan/commit/5f8b3e0609ae1bcf482c54527dfa5cc85e3b3b22)

- _(term)_ more improvments to ipc structure and event handling [_(ad24595)_](https://github.com/robgonnella/r-lanscan/commit/ad2459525939fe87a0a7eb806cb8f2f9453607a1)

- _(term)_ more improvements to renderer instantiation [_(3e77903)_](https://github.com/robgonnella/r-lanscan/commit/3e77903db92509b7363e03b427a56e518cb7a1b5)

- _(term)_ removes unused field in renderer [_(e8c4835)_](https://github.com/robgonnella/r-lanscan/commit/e8c4835ef6d1bc770cd8c3224d9ef488cda94d90)

- _(term)_ improves separation of concerns [_(e9348c0)_](https://github.com/robgonnella/r-lanscan/commit/e9348c0b256687b9f97ea047cada1266895ab14c)

- _(term)_ renames event modules and separates shell executor [_(21202f9)_](https://github.com/robgonnella/r-lanscan/commit/21202f9c36ccd0d19f0234134ef335ed16854890)

### 🧪 Testing

- adds tests for scrollview and logsview [_(a18aab7)_](https://github.com/robgonnella/r-lanscan/commit/a18aab79603aef05969132ffa434010a62d1314a)

# [0.4.0](https://github.com/robgonnella/r-lanscan/releases/tag/v0.4.0) - 2026-01-18

### 🚀 Features

- adds basic service lookup for open ports [_(57b3712)_](https://github.com/robgonnella/r-lanscan/commit/57b3712fffee28226c0435a4830da4ab3b08b72a)

### 🐛 Bug Fixes

- fixes issue with loading default config [_(17c5cf1)_](https://github.com/robgonnella/r-lanscan/commit/17c5cf1bff3190d6d3352041b7749b591815aece)

- fixes minor issue in cli [_(d0e8d50)_](https://github.com/robgonnella/r-lanscan/commit/d0e8d50a386722266872297ea392a9bf81159c2a)

- fixes panics caused by target parsing [_(7415c43)_](https://github.com/robgonnella/r-lanscan/commit/7415c4395bf6471bae0f00d505346f70db101a97)

### 🚜 Refactor

- uses builder pattern for packet construction [_(f0f0ac7)_](https://github.com/robgonnella/r-lanscan/commit/f0f0ac71c4a4ce5dc2d5ebd2b03134453e67b8ba)

- refactors scan message types [_(d8dfc56)_](https://github.com/robgonnella/r-lanscan/commit/d8dfc5600a991cc35d38638462d4943ed0a34a89)

- _(term)_ stores pre-computed sorted device list in state [_(fb4f9fb)_](https://github.com/robgonnella/r-lanscan/commit/fb4f9fb909522bdd19b11d3f3afe6b116e0cfbbf)

- simplify types and improve performance [_(1c7e151)_](https://github.com/robgonnella/r-lanscan/commit/1c7e151fe846a1f864d3120f86aa1a50eb1af74d)

- removes all remaining unwraps from hot paths [_(ff8ecd0)_](https://github.com/robgonnella/r-lanscan/commit/ff8ecd0eae78f215a37ef6fe5e57528f8fd01ee8)

- refactors monolithic reducer [_(85c85f9)_](https://github.com/robgonnella/r-lanscan/commit/85c85f9416a859c255d92504651fcf5f5bd7f39f)

### ⚡ Performance

- improves performance in term ui app [_(2497df3)_](https://github.com/robgonnella/r-lanscan/commit/2497df34d70ecd4ad6fa7a41052968ee774a89bb)

- performance improvements [_(5565011)_](https://github.com/robgonnella/r-lanscan/commit/5565011e945bcdcdf2d44e9b76eb04712bcd8fc3)

### 📚 Documentation

- updates doc comments and adds CONTRIBUTING.md [_(85c370f)_](https://github.com/robgonnella/r-lanscan/commit/85c370fcfe5da54f91066c5af48e761262526253)

# [0.3.0](https://github.com/robgonnella/r-lanscan/releases/tag/v0.3.0) - 2026-01-04

### 🚀 Features

- publish binaries (#79) [_(b614c92)_](https://github.com/robgonnella/r-lanscan/commit/b614c92a739dc26d5ebdfd923acdda1c69f0953a)

