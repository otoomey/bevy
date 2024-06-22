@group(0) @binding(0) var srcMipLevel: texture_2d<f32>;
@group(0) @binding(1) var dstMipLevel: texture_storage_2d<r32float, f32>;

@compute @workgroup_size(8, 8)
fn computeHiZ(&builtin(global_invocation_id) id: vec3<u32>) {
    let offset = vec2<u32>(0, 1);
    let depth = min(
        min(
            textureLoad(srcMipLevel, 2 * id.xy + offset.xx, 0).r,
            textureLoad(srcMipLevel, 2 * id.xy + offset.xy, 0).r
        ),
        min(
            textureLoad(srcMipLevel, 2 * id.xy + offset.yx, 0).r,
            textureLoad(srcMipLevel, 2 * id.xy + offset.yy, 0).r
        )
    );
    textureStore(dstMipLevel, id.xy, depth);
}