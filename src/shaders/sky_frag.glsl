#version 140

uniform samplerCube cubemap;

in vec3 tex_coords;

out vec4 f_color;

void main(void) {
	f_color = texture(cubemap, tex_coords);
}
