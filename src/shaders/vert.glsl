#version 150 core

in vec4 a_Pos;
in vec3 a_Color;

uniform Transform {
	mat4 u_Transform;
};

out vec4 v_Color;

void main(void) {
	v_Color = vec4(a_Color, 1.0);
	gl_Position = a_Pos * u_Transform;
}
