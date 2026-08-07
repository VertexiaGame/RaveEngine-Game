#define_import_path bevy_open_world::common

const UI0 = u32(1597334673);
const UI1 = u32(3812015801);
const UI3 = vec3u(UI0, UI1, u32(2798796415));
const UIF = (1.0 / f32(0xffffffff));

fn linearstep(s: f32, e: f32, v: f32) -> f32 {
    return clamp((v - s) * (1.0 / (e - s)), 0.0, 1.0);
}

fn linearstep0(e: f32, v: f32) -> f32 {
    return min(v * (1.0 / e), 1.0);
}

fn remap(v: f32, s: f32, e: f32) -> f32 {
    return (v - s) / (e - s);
}

fn mod_tile(v: vec3f, tile: f32) -> vec3f {
    return (v % vec3f(tile) + vec3f(tile)) % vec3f(tile);
}

fn hash13(p3_in: vec3f) -> f32 {
    let i = vec3i(floor(p3_in * 1000.0));
    let u = vec3u(bitcast<u32>(i.x) & 0xffffu, bitcast<u32>(i.y) & 0xffffu, bitcast<u32>(i.z) & 0xffffu);
    var n = u.x ^ (u.y * 1597334677u) ^ (u.z * 3812015801u);
    n = (n ^ (n >> 16u)) * 0x7feb352du;
    n = (n ^ (n >> 15u)) * 0x846ca68bu;
    n = n ^ (n >> 16u);
    return f32(n) * (1.0 / 4294967295.0);
}

fn value_hash(p3_in: vec3f) -> f32 {
    let i = vec3i(floor(p3_in));
    let u = vec3u(bitcast<u32>(i.x) & 0xffffu, bitcast<u32>(i.y) & 0xffffu, bitcast<u32>(i.z) & 0xffffu);
    var n = u.x ^ (u.y * 1597334677u) ^ (u.z * 3812015801u);
    n = (n ^ (n >> 16u)) * 0x7feb352du;
    n = (n ^ (n >> 15u)) * 0x846ca68bu;
    n = n ^ (n >> 16u);
    return f32(n) * (1.0 / 4294967295.0);
}

fn hash_based_noise(x: vec3f, tile: f32) -> f32 {
    let p = floor(x);
    var f = fract(x);
    f = f * f * (3.0 - 2.0 * f);

    return mix(
        mix(
            mix(
                value_hash(mod_tile(p, tile)),
                value_hash(mod_tile(p + vec3f(1.0, 0.0, 0.0), tile)),
                f.x
            ),
            mix(
                value_hash(mod_tile(p + vec3f(0.0, 1.0, 0.0), tile)),
                value_hash(mod_tile(p + vec3f(1.0, 1.0, 0.0), tile)),
                f.x
            ),
            f.y
        ),
        mix(
            mix(
                value_hash(mod_tile(p + vec3f(0.0, 0.0, 1.0), tile)),
                value_hash(mod_tile(p + vec3f(1.0, 0.0, 1.0), tile)),
                f.x
            ),
            mix(
                value_hash(mod_tile(p + vec3f(0.0, 1.0, 1.0), tile)),
                value_hash(mod_tile(p + vec3f(1.0, 1.0, 1.0), tile)),
                f.x
            ),
            f.y
        ),
        f.z
    );
}

fn hash33(p: vec3f) -> vec3f {
    let i = vec3i(floor(p));
    let u = vec3u(bitcast<u32>(i.x) & 0xffffu, bitcast<u32>(i.y) & 0xffffu, bitcast<u32>(i.z) & 0xffffu);

    var n1 = u.x ^ (u.y * 1597334677u) ^ (u.z * 3812015801u);
    n1 = (n1 ^ (n1 >> 16u)) * 0x7feb352du;
    n1 = (n1 ^ (n1 >> 15u)) * 0x846ca68bu;
    n1 = n1 ^ (n1 >> 16u);

    var n2 = u.y ^ (u.z * 1597334677u) ^ (u.x * 3812015801u);
    n2 = (n2 ^ (n2 >> 16u)) * 0x7feb352du;
    n2 = (n2 ^ (n2 >> 15u)) * 0x846ca68bu;
    n2 = n2 ^ (n2 >> 16u);

    var n3 = u.z ^ (u.x * 1597334677u) ^ (u.y * 3812015801u);
    n3 = (n3 ^ (n3 >> 16u)) * 0x7feb352du;
    n3 = (n3 ^ (n3 >> 15u)) * 0x846ca68bu;
    n3 = n3 ^ (n3 >> 16u);

    return -1.0 + 2.0 * vec3f(
        f32(n1) * (1.0 / 4294967295.0),
        f32(n2) * (1.0 / 4294967295.0),
        f32(n3) * (1.0 / 4294967295.0)
    );
}

fn gradient_noise(x: vec3f, freq: f32) -> f32 {
    let p = floor(x);
    let w = fract(x);

    let u = w * w * w * (w * (w * 6.0 - 15.0) + 10.0);

    let ga = hash33(mod_tile(p + vec3f(0.0, 0.0, 0.0), freq));
    let gb = hash33(mod_tile(p + vec3f(1.0, 0.0, 0.0), freq));
    let gc = hash33(mod_tile(p + vec3f(0.0, 1.0, 0.0), freq));
    let gd = hash33(mod_tile(p + vec3f(1.0, 1.0, 0.0), freq));
    let ge = hash33(mod_tile(p + vec3f(0.0, 0.0, 1.0), freq));
    let gf = hash33(mod_tile(p + vec3f(1.0, 0.0, 1.0), freq));
    let gg = hash33(mod_tile(p + vec3f(0.0, 1.0, 1.0), freq));
    let gh = hash33(mod_tile(p + vec3f(1.0, 1.0, 1.0), freq));

    let va = dot(ga, w - vec3f(0.0, 0.0, 0.0));
    let vb = dot(gb, w - vec3f(1.0, 0.0, 0.0));
    let vc = dot(gc, w - vec3f(0.0, 1.0, 0.0));
    let vd = dot(gd, w - vec3f(1.0, 1.0, 0.0));
    let ve = dot(ge, w - vec3f(0.0, 0.0, 1.0));
    let vf = dot(gf, w - vec3f(1.0, 0.0, 1.0));
    let vg = dot(gg, w - vec3f(0.0, 1.0, 1.0));
    let vh = dot(gh, w - vec3f(1.0, 1.0, 1.0));

    return va +
           u.x * (vb - va) +
           u.y * (vc - va) +
           u.z * (ve - va) +
           u.x * u.y * (va - vb - vc + vd) +
           u.y * u.z * (va - vc - ve + vg) +
           u.z * u.x * (va - vb - ve + vf) +
           u.x * u.y * u.z * (-va + vb + vc - vd + ve - vf - vg + vh);
}

fn voronoi(x: vec3f, tile: f32) -> f32 {
    let p = floor(x);
    let f = fract(x);

    var res = 100.0;

    for (var k = -1.0; k < 1.1; k += 1.0) {
        for (var j = -1.0; j < 1.1; j += 1.0) {
            for (var i = -1.0; i < 1.1; i += 1.0) {
                let b = vec3f(i, j, k);
                let c = mod_tile(p + b, tile);

                let r = vec3f(b) - f + hash13(c);
                let d = dot(r, r);

                res = min(res, d);
            }
        }
    }

    return 1.0 - res;
}

fn tilable_voronoi(p: vec3f, octaves: i32, _freq: f32) -> f32 {
    var freq = _freq;
    var amplitude = 1.0;
    var noise = 0.0;
    var w = 0.0;

    for (var i = 0; i < octaves; i++) {
        noise += amplitude * voronoi(p * freq, freq);
        freq *= 2.0;
        w += amplitude;
        amplitude *= 0.5;
    }

    return noise / w;
}

fn tilable_perlin_fbm(p: vec3f, octaves: i32, _freq: f32) -> f32 {
    var freq = _freq;
    var amplitude = 1.0;
    var noise = 0.0;
    var w = 0.0;

    for (var i = 0; i < octaves; i++) {
        noise += amplitude * hash_based_noise(p * freq, freq);
        freq *= 2.0;
        w += amplitude;
        amplitude *= 0.5;
    }

    return noise / w;
}