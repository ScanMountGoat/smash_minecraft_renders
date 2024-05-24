# smash_minecraft_renders
A Rust library and CLI program for creating Minecraft renders for Smash Ultimate using image processing techniques.

## Generating Correct Input Images
The input skin textures are expected to be in the Minecraft Java layout, which is also used for Smash Ultimate. See the included `sample.png` for reference.  

Input textures should be precorrected using the following formula to avoid over exposing the render and more closely match Smash Ultimate's textures.
This is equivalent to a levels adjustment with highlight output set to `184` and midtone output set to `1.386`.   
`output.rgb = (input.rgb ^ 0.72) * 0.72`  

The generated UV map should have vertices snapped to pixel corners and be exported without antialiasing or any sort of dithering to avoid distorting the texture sampling.  
This can be achieved in Blender by setting view transform to `Raw`, dither to `0.00`, and samples to `1`. See the provided `uvs.png` for the expected dimensions and layout.  

## CLI Usage
`minecraft_render.exe [FLAGS] --skin <sample.png>`  
`minecraft_render.exe -h` for a list of arguments and options.    

Creates the following files:  
```
chara_3_custom.png
chara_4_custom.png
chara_6_custom.png
output.png
```

