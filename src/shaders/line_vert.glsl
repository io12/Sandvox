#version 140

uniform mat4 matrix;

in vec3 pos;

void main(void) {
	gl_Position = matrix * vec4(pos, 1.0);
}
