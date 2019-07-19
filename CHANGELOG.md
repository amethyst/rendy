# Changelog

## 0.3.2

* Add dyn group api to subpass builder ([#169])
* Removed uses of FnvHashMap for Rust standard library's HashMap ([#170])
* Fix potentially unsound Factory::drop ([#173])
* Check that surface presentation supported by the queue ([#177])

## 0.3.1

* Fix short-circuting problem in render pass

## 0.3

* Remove waiting on zero fences ([#138])
* Raw string source shader support ([#141])
* Fix memory allocator asserts ([#155], [#156])
* Add frames in flight to graph context ([#146])
* Fix double-panic on resource destruction ([#150])
* Add more ways to construct DescriptorRanges ([#153])
* Reduce logging level ([#161])
* Allow render pass to use surface as an attachment ([#164])
* Add `premultiply_alpha` option to `ImageTextureConfig` ([#143])
* Mark potentially unsafe command recording methods ([#165])

[#138]: https://github.com/amethyst/rendy/pull/138
[#141]: https://github.com/amethyst/rendy/pull/141
[#143]: https://github.com/amethyst/rendy/pull/143
[#146]: https://github.com/amethyst/rendy/pull/146
[#150]: https://github.com/amethyst/rendy/pull/150
[#153]: https://github.com/amethyst/rendy/pull/153
[#155]: https://github.com/amethyst/rendy/pull/155
[#156]: https://github.com/amethyst/rendy/pull/156
[#161]: https://github.com/amethyst/rendy/pull/161
[#164]: https://github.com/amethyst/rendy/pull/164
[#165]: https://github.com/amethyst/rendy/pull/165
[#169]: https://github.com/amethyst/rendy/pull/169
[#170]: https://github.com/amethyst/rendy/pull/170
[#173]: https://github.com/amethyst/rendy/pull/173
[#177]: https://github.com/amethyst/rendy/pull/177
