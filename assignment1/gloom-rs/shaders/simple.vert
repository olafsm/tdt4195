#version 430 core

in layout(location=0) vec3 position;
in layout(location=1) vec4 in_color;
in layout(location=2) vec3 in_normal;
uniform layout(location=3) mat4 trans_mat;

out vec4 vColor;
out vec3 vNormal;

void main()
{
    vColor = in_color;
    vNormal = in_normal;
    gl_Position = trans_mat*vec4(position, 1.0f);
}