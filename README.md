# smash_minecraft_renders
A Rust library and CLI program for creating Minecraft renders for Smash Ultimate using image processing techniques.
Input textures should be precorrected using the following formula for best results.  
`output.rgb = (input.rgb ^ 0.72) * 0.72`

# Usage
`smash_minecraft_renders.exe <Minecraft skin texture>`
`smash_minecraft_renders.exe sample.png`
