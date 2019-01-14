#version 140

uniform mat4 matrix;

in vec3 pos;
in uint voxel_type;

out vec4 v_color;

void main(void) {
	gl_Position = matrix * vec4(pos, 1.0);
	switch (voxel_type) {
	case 1u: // Sand
		if (int(pos.x) % 2 == 0) {
			v_color = vec4(0.926, 0.785, 0.684, 1.0);
		} else if (int(pos.z) % 2 == 0) {
			v_color = vec4(0.626, 0.585, 0.484, 1.0);
		} else {
			v_color = vec4(0.426, 0.385, 0.284, 1.0);
		}
		break;
	default:
		v_color = vec4(1.0, 1.0, 1.0, 1.0);
	}
}
