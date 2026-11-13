// WriterCraft original filled passage tag; exact polygon source: tag.svg.
// Five-point silhouette and a transparent eye, with native pixel antialiasing.
const TAG_GEOMETRY: IconGeometry = IconGeometry {
    name: "writercraft-tag",
    vertices: &[
        [3.0, 3.0, -1.0, -1.0],
        [12.0, 3.0, std::f32::consts::SQRT_2 - 1.0, -1.0],
        [21.0, 12.0, std::f32::consts::SQRT_2, 0.0],
        [12.0, 21.0, 0.0, std::f32::consts::SQRT_2],
        [3.0, 12.0, -1.0, std::f32::consts::SQRT_2 - 1.0],
        [6.0, 5.0, 1.0, 1.0],
        [8.0, 5.0, -1.0, 1.0],
        [8.0, 7.0, -1.0, -1.0],
        [6.0, 7.0, 1.0, -1.0],
    ],
    triangles: &[
        0, 1, 6, 0, 6, 5, 1, 2, 7, 1, 7, 6, 2, 3, 4, 2, 4, 8, 2, 8, 7, 4, 0, 5, 4, 5, 8,
    ],
    contour_edges: &[
        [0, 1],
        [1, 2],
        [2, 3],
        [3, 4],
        [4, 0],
        [5, 8],
        [8, 7],
        [7, 6],
        [6, 5],
    ],
};
