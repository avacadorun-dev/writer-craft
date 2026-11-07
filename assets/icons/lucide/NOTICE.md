# Lucide utility icon subset

Source: https://github.com/lucide-icons/lucide
Pinned upstream commit: `53fab88ff9d1b48f4704b5b5b560d6b4d560bf4c`.
Original geometry: `https://raw.githubusercontent.com/lucide-icons/lucide/53fab88ff9d1b48f4704b5b5b560d6b4d560bf4c/icons/<name>.svg`.
The complete upstream LICENSE is retained, including ISC copyright and Feather-derived MIT notices. WriterCraft's separate monk brand mark is not a Lucide utility icon.

All30 utility icons use this one library. Original SVGs are unmodified. Native vector data is generated mechanically from their SVG paths/circles/rectangles; no icon was redrawn by hand. Geometry uses the original24×24 coordinate space, stroke width2 and round caps/joins. A generation-only Rust tool uses kurbo0.13.1 for SVG curves and Lyon for fill/stroke tessellation at0.01 source-unit tolerance. Native rendering adds a one-physical-pixel antialias fringe at the actual display scale. No SVG runtime, font asset or application dependency is added.

`native_geometry.rs` is the generated selected subset (5942 unique vertices, 5823 triangles), with contour normals/edges for native antialiasing. Regeneration: parse the pinned source elements into equivalent SVG paths, preserve fill/stroke attributes, tessellate with round caps/joins and tolerance0.01, merge coincident vertices, derive exterior edges from triangle adjacency and regenerate the exhaustive Icon mapping. The isolated generator source and lockfile were used for this conversion; they are development tooling, not application dependencies. Verify original SVG SHA-256 values below before regeneration.

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
