# Benchmark corpus

Real doc comments from pinned sources, one JSON array of strings per size. [tools/bench/corpus.sh](../../tools/bench/corpus.sh) remakes them from the same pins.

| Size | Comments | Source | Pinned at | License |
| --- | --- | --- | --- | --- |
| small | 580 | [microsoft/tsdoc](https://github.com/microsoft/tsdoc)'s `tsdoc/src`, tests left out | commit `87457daa029f68ad955422a6ae3f7ce2be59fc11` | MIT, Copyright (c) Microsoft Corporation |
| medium | 1327 | [TypeScript](https://github.com/microsoft/TypeScript)'s `lib/lib.es*.d.ts` | npm `typescript@5.9.3`, integrity `sha512-jl1vZzPDinLr9eUt3J/t7V6FgNEw9QjvBPdysz9KfQDD41fQrC2Y4vKQdiaUpFT4bXlb1RHhLpp8wtm6M5TgSw==` | Apache-2.0, Copyright (c) Microsoft Corporation |
| large | 5604 | TypeScript's `lib/lib.dom.d.ts` | the same | Apache-2.0, Copyright (c) Microsoft Corporation |

The comments are copied unchanged, but for CRLF line ends made LF. TypeScript's library comments are JSDoc rather than TSDoc, so they also measure how each parser handles text TSDoc reports on, like an `@param` without a hyphen.
