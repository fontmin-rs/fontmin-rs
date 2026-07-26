# fontmin-rs patch notes

This directory contains `allsorts` 0.17.0 from crates.io
(`83b5535c25712f8ba509f2e9f134581e080ff8b6699f3fbf12bfda95d82fcb36`).

The local patch makes CFF and CFF2 INDEX parsing reject malformed offset
arrays before an object is sliced:

- checked arithmetic is used for the offset-array length;
- the first offset must be one, as required by the CFF format;
- subsequent offsets must be non-decreasing;
- object access uses checked subtraction and slice lookup.

`tests/cff_index.rs` covers zero, descending, empty-object, and oversized CFF2
INDEX cases. The library's upstream unit-test target is disabled because the
published crate omits font files referenced by those tests; the dedicated
integration test remains enabled and is run by the repository test command.

Remove this override after an upstream release contains equivalent validation
and the repository regression corpus passes against that release.
