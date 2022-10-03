#version 430 core

out vec4 color;

in vec4 vColor;
in vec3 vNormal;

void main()
{   
    vec3 lightDirection = normalize(vec3(0.8, -0.5, 0.6));
    vec3 scaledColor = vec3(vColor.r,vColor.g,vColor.b)*max(0,dot(vNormal, -lightDirection));
    color = vec4(scaledColor, vColor.a);
}