#version 140

in vec3 vColor;
out vec4 f_color;

void main(void) {
	f_color = vec4(vColor, 1.0);
}
