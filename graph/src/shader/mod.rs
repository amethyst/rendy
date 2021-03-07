const SHADER_SLOTS: [Option<usize>; 32] = [
    Some(0), // 0, 0x00000001: Vertex shader
    Some(1), // 1, 0x00000002: Hull (tesselation) shader
    Some(2), // 2, 0x00000004: Domain (tesselation) shader
    Some(3), // 3, 0x00000008: Geometry shader
    Some(4), // 4, 0x00000010: Fragment shader
    Some(5), // 5, 0x00000020: Compute shader
    Some(6), // 6, 0x00000040: Task shader
    Some(7), // 7, 0x00000080: Mesh shader
    None, // 8, 0x00000100
    None, // 9, 0x00000200
    None, // 10, 0x00000400
    None, // 11, 0x00000800
    None, // 12, 0x00001000
    None, // 13, 0x00002000
    None, // 14, 0x00004000
    None, // 15, 0x00008000
    None, // 16, 0x00010000
    None, // 17, 0x00020000
    None, // 18, 0x00040000
    None, // 19, 0x00080000
    None, // 20, 0x00100000
    None, // 21, 0x00200000
    None, // 22, 0x00400000
    None, // 23, 0x00800000
    None, // 24, 0x01000000
    None, // 25, 0x02000000
    None, // 26, 0x04000000
    None, // 27, 0x08000000
    None, // 28, 0x10000000
    None, // 29, 0x20000000
    None, // 30, 0x40000000
    None, // 31, 0x80000000
];

pub struct Program {

}
