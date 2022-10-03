#version 430 core

in layout(location=0) vec3 position;
in layout(location=1) vec4 in_color;
uniform layout(location=3) mat4 trans_mat;

out vec4 vColor;

void main()
{
    vColor = in_color;
    gl_Position = trans_mat*vec4(position, 1.0f);
}