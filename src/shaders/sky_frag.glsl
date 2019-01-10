#version 140

uniform samplerCube skybox;

in vec3 tex_coords;

out vec4 f_color;

void main(void) {
	f_color = texture(skybox, tex_coords);
}
