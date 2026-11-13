// SPDX-License-Identifier: GPL-2.0-only
uniform float Saturation < ui_type = "slider"; ui_min = 0.0; ui_max = 2.0; ui_label = "Saturation"; > = 1.0;
uniform float Contrast < ui_type = "slider"; ui_min = 0.5; ui_max = 1.5; ui_label = "Contrast"; > = 1.0;

texture2D BackBuffer : COLOR;
sampler2D ColorSampler { Texture = BackBuffer; };

void FullscreenVertex(uint vertexId : SV_VertexID, out float4 position : SV_Position, out float2 uv : TEXCOORD)
{
    uv = float2((vertexId << 1) & 2, vertexId & 2);
    position = float4(uv * float2(2.0, -2.0) + float2(-1.0, 1.0), 0.0, 1.0);
}

float4 ColorPixel(float4 position : SV_Position, float2 uv : TEXCOORD) : SV_Target
{
    float4 color = tex2D(ColorSampler, uv);
    float luminance = dot(color.rgb, float3(0.2126, 0.7152, 0.0722));
    color.rgb = saturate((lerp(luminance.xxx, color.rgb, Saturation) - 0.5) * Contrast + 0.5);
    return color;
}

technique ArachnidColor < ui_label = "Arachnid Color"; ui_tooltip = "Optional color controls. Neutral at the default values."; >
{
    pass ColorPass
    {
        VertexShader = FullscreenVertex;
        PixelShader = ColorPixel;
    }
}
