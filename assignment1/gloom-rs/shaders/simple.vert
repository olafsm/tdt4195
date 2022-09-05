#version 430 core

in vec3 position;

void main()
{
    gl_Position = vec4(position, 1.0f);
    gl_Position *= vec4(-1.0f,-1.0f,1.0f,1.0f);
}