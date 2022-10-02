#version 430 core

out vec4 color;

in vec4 vColor;
//in vec3 vNormal;
void main()
{   
    color = vColor;
    //color = vec4(vNormal,1.0f);
}