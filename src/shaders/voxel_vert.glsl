#version 140

uniform mat4 matrix;

in vec3 pos;
in vec4 color;

out vec4 v_color;

void main(void) {
	gl_Position = matrix * vec4(pos, 1.0);
	v_color = color;
}
