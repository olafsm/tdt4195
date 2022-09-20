#version 430 core

in layout(location=0) vec3 position;
in layout(location=1) vec4 in_color;

out vec4 vColor;

void main()
{
    vColor = in_color;
    float a = 1;
    float e = 1;
    
    float b = 0;
    float d = 0;

    float c = 0;
    float f = 0;

    mat4 matrix = mat4(
        a,b,0,c,
        d,e,0,f,
        0,0,1,0,
        0,0,0,1
    );
    gl_Position = vec4(position, 1.0f);
    gl_Position *= matrix;
    //gl_Position *= vec4(-1.0f,-1.0f,1.0f,1.0f);
}