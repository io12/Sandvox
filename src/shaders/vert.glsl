#version 140

uniform mat4 matrix;

in vec3 pos;
in vec3 color;

out vec3 vColor;

void main(void) {
	gl_Position = matrix * vec4(pos, 1.0);
	vColor = color;
}
