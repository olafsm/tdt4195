#version 430 core

out vec4 color;

void main()
{   
    if (  gl_FragCoord.x < 200){
        color = vec4(0.7f,   0.3f, 0.5f, 1.0f);
    }
    else {
        color = vec4(0.7f, 0.8f, 0.3f, 1.0f);
    }

}