# Lucide utility icon subset

Source: https://github.com/lucide-icons/lucide
Pinned upstream commit: `53fab88ff9d1b48f4704b5b5b560d6b4d560bf4c`.
Original geometry: `https://raw.githubusercontent.com/lucide-icons/lucide/53fab88ff9d1b48f4704b5b5b560d6b4d560bf4c/icons/<name>.svg`.
The complete upstream LICENSE is retained, including ISC copyright and Feather-derived MIT notices. WriterCraft's separate monk brand mark is not a Lucide utility icon.

All 52 utility icons use this one library. Original SVGs are unmodified. Native vector data is generated mechanically from their SVG paths/circles/rectangles; no icon was redrawn by hand. Geometry uses the original24×24 coordinate space, stroke width2 and round caps/joins. A generation-only Rust tool uses kurbo0.13.1 for SVG curves and Lyon for fill/stroke tessellation at0.01 source-unit tolerance. Native rendering adds a one-physical-pixel antialias fringe at the actual display scale. No SVG runtime, font asset or application dependency is added.

`native_geometry.rs` is the generated selected subset (10,163 unique vertices, 9,924 triangles), with contour normals/edges for native antialiasing. Regeneration: parse the pinned source elements into equivalent SVG paths, preserve fill/stroke attributes, tessellate with round caps/joins and tolerance0.01, merge coincident vertices, derive exterior edges from triangle adjacency and regenerate the exhaustive Icon mapping. The isolated generator source and lockfile were used for this conversion; they are development tooling, not application dependencies. Verify original SVG SHA-256 values below before regeneration.

| WriterCraft variant | Official Lucide file | SHA-256 |
|---|---|---|
| Home | house.svg | `28216c479005731ef5e382e7d8627eda2b129b019eb1f3d9de9f5915576d448e` |
| Book | book-open.svg | `9b9bed0ff057283d5c5c5a1c71f8d3e927db26a56639603a92fdf7b25961510e` |
| Folder | folder.svg | `f0d92b94b797a8ab7d4c4ae33c3236c47f64068351b4e14bdb5014ee42898a39` |
| Document | file-text.svg | `52ca55846c202335fb870c8aa23e9345630ec10489303756d62e750f7109605c` |
| Plus | plus.svg | `7f6af73bf1ff6c4bca3f18351c8d1bdec6749c0c2530c4de5da85d520c21df17` |
| Search | search.svg | `283d371c2e433817bb9c0c8310caa6c77fa4177c0f4f1168d9c83b97af7389dc` |
| Sidebar | panel-left.svg | `029f67b9e3eeb7ddf7c7c499b174c50388725e8ea50d4ea59ed7dbaf8d61cb82` |
| Assistant | message-square-text.svg | `b2564066702194c600d91b6def8e897cdbf4c6205c80d39e459ff0a12e13950d` |
| Mic | mic.svg | `9b940cd735d1ffa270ba670c4e677d9a09400f3fa81d6b3f405b12a3b67a59a8` |
| Settings | settings.svg | `0ae27fd0f81999229e3127ac96c5b32edfea448e291d509e76212b917551d66b` |
| Import | download.svg | `3daeee13ded5a3afa2198b1bb4e2262d163922f97d96ac6ff9953e8ed4fe039d` |
| Export | upload.svg | `42816ee1fa0e9d3b82272a97615c1e562b0e9de64780ebf21963bc1db0ceffd1` |
| Save | save.svg | `ee9d56a7fec4b20dd6689546d41f68219ea8cbd67f99bd17a1bdaff5be6edb53` |
| Undo | undo-2.svg | `efad2edbbd2be6038cc23fd64df2d012fad5cc6460dfbf227cfa3d990e9e897f` |
| Redo | redo-2.svg | `4cbd2cdebc70b6b7979c681ed3f313d9cbb102465dcf7d0d2d6a72c937b344bc` |
| Check | check.svg | `7f33acc9a77a61659531044525fc008edebe215bf4dcf1c789c8674ad3277db0` |
| ChevronRight | chevron-right.svg | `2758143d7b2434e4aa7307dfd34405c87909ff4052f21b5f3f40d45224b4f19b` |
| More | ellipsis.svg | `4f495cc72013ffdfec677f03b33a150f7b4dd741979283fd6853a09024bca112` |
| Close | x.svg | `4a9cdab38fbb96162e7dace28e33f4ca0e49d8963a6162abc3d4691b7d675117` |
| History | rotate-ccw-clock.svg | `a98ff758157910b9fc9d42d5d5dc80c2f4e037afa5f8ea4dee5f5649f03d438d` |
| Board | panels-top-left.svg | `321a2a5500f2b814a2e62eb73c4b01dc65c7334fe2e6f7d77517239383e26bb1` |
| Trash | trash.svg | `27299f69ad7c6272be64b1b8e2d48cbd6dcf0ef0d4f92827a1affa945c91700e` |
| ArrowRight | arrow-right.svg | `288397fdb73bb4f783cf9eabb726ff47357df1d4e3f1201dd748090579f108b9` |
| Shield | shield-check.svg | `8d4fcdbde5bba4abb1a2f6fe53e37ceb6343a1d5449a9d88f6f97c1971d26c4e` |
| Sun | sun.svg | `a3955c8b0425fcc5c9ba12d2bdd77fe53b2cd5dd05e7742a56f342e996fb2ef8` |
| Moon | moon.svg | `205b959e940d5841f0ce1d09c933153da9d9c94130a54d5cc0c68c2af1d403da` |
| Palette | palette.svg | `8e7d3fde990cffd10469ba351f27c50202b057a9bb9af5626322e53fd3abc78c` |
| Keyboard | keyboard.svg | `f509925ca81964ed06251ad4702e989bb81a566ffc74d15c2226b4f07ded67be` |
| Plug | plug.svg | `11ba780e4b6f15d37906a8a73537144b59720333d7a9fc8048a437d0389422f8` |
| TextCursor | text-cursor.svg | `1224fd2e9488b38a4cdc743ea2ada7db7fe0a31e53d44758e5a5ac45f7971177` |

## Editor authoring extension

The editor subset below comes from the same pinned upstream revision and license. Canonical file names follow that source tree (for example, the bold-format glyph is `letter-b`, indentation uses `list-indent-*`, and warning uses `triangle-exclamation-point`). Original SVGs are unmodified. The existing generation-only Kurbo/Lyon executable produced the stroke triangles; mechanical vertex merging and exterior contour normals produce the same native antialias fringe. No runtime dependency or local Cargo build was added.

| WriterCraft variant | Official Lucide file | SHA-256 |
|---|---|---|
| Bold | letter-b.svg | `c54339aca9fa8095097c3055126c37838ac2f2ba54cec3ba302a0b5d0cc44379` |
| Italic | italic.svg | `dcce4f9719febb8b04b030133c1d4ad82f68d13932ac806fbee620422000ac83` |
| Underline | underline.svg | `f83a0a4cc2f4e3a8616dd27b8a4c5a9e60e577a9791c08353eaa2fdc8370f6cb` |
| Strike | strikethrough.svg | `1f52d0951beb1f5ddd888cf0e432694ed18ed3020d356f5d0b7ceb16bc371728` |
| Code | code-xml.svg | `a6d2110227be375be21da7324d3c09262c1bf019d06bdeab356d515342ab7cce` |
| Quote | quote.svg | `3c7a2f8443ae5c88cb3f35fdcc27a037a1af13fe35be28eb8cabded8672a9ac1` |
| BulletList | list.svg | `ff97a7379eb962ddc3863bb59f0f45875ac8da91aac8a38a67361e7118d2d627` |
| OrderedList | list-ordered.svg | `df549e10677b86c9a9f07c0c696b40a4937019212e73bc982a4e8dae7c557d8f` |
| Indent | list-indent-increase.svg | `56932539f1c6aecbf0e82a8b401cd937c57e886d968c41230ad6df54f7446a2d` |
| Outdent | list-indent-decrease.svg | `b67137c021b93811c6741c0a4a4487471585319d65c00b12320e678af36f390f` |
| Table | table-2.svg | `7b1808e170670d644af828bed77022fc8baba27a654e0805f3fb98e0bec62077` |
| Formula | sigma.svg | `4479d09dc10cbfcefc7f112ce6fd825da57779aa60d6d83f62882aa78421da18` |
| Image | image-plus.svg | `19a5bef8f17d2397b2c41422553f53746e7c8ad9886a11e7ad397c5371368ec0` |
| Diagram | workflow.svg | `ae99ea8f56a2835a2cca35b2fbb57643ab73443610970dba23e24bd9df75cfec` |
| Info | info.svg | `bc977a64eb96f3e9c1041ffd09a3fceb70e3e65c02b571f76064062ed31f3cb9` |
| Idea | lightbulb.svg | `e33bbec15b1e64cca958e086850f951c0dc2201cd5778bb3695fda3cb6ea457d` |
| Warning | triangle-exclamation-point.svg | `4866f38b8560d410f21e3226413e0b77997b6dfbb6931fadfe0a0d5aef9ffeb4` |
| Todo | list-todo.svg | `8620dcb86d9d27e0eeebc627b6a0f995ff3bb0264fd3c12ad003401981771848` |
| Link | link.svg | `4f3237183dcaabfac92f4e94c776a9a9e45c3c5e77a921061a6b34b61eb5413f` |
| Minus | minus.svg | `a0c743ab6dbf545d8a6e19ef3874f48ede686ce68d25e231bd81f540d97b1f19` |
| Focus | focus.svg | `d52fde05ccbd7bf629e05daeb8eadf824c85333aeca78e674db01493bc8b1f10` |
| Copy | copy.svg | `ea80e566c7a12628a447cb53179b19aea4f60b9143e2becd0d0f20bf260e5718` |
