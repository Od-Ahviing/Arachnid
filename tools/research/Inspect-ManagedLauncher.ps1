param(
    [Parameter(Mandatory = $true)]
    [string] $Path,
    [switch] $Resources
)

$ErrorActionPreference = 'Stop'
$resolvedPath = (Resolve-Path -LiteralPath $Path).ProviderPath
$file = Get-Item -LiteralPath $resolvedPath
$fileHash = Get-FileHash -LiteralPath $resolvedPath -Algorithm SHA256
if ($file.Length -gt 2MB -or $fileHash.Hash -ne '1B957CBA6E561982DD6688B7E1EF7363C416C3A1B463FCF624700329D518E831') {
    throw 'This research tool accepts only the inspected local launcher hash, not arbitrary PE files.'
}
$referenceAssemblies = (Get-ChildItem -LiteralPath (Join-Path $PSHOME 'ref') -Filter '*.dll').FullName

Add-Type -ReferencedAssemblies $referenceAssemblies -TypeDefinition @'
using System;
using System.IO;
using System.Linq;
using System.Collections.Generic;
using System.Reflection;
using System.Reflection.Emit;
using System.Reflection.Metadata;
using System.Reflection.Metadata.Ecma335;
using System.Reflection.PortableExecutable;
using System.Resources;
using System.Text;
using System.Text.RegularExpressions;

public static class LauncherMetadataInspection
{
    public static void InspectResources(string path)
    {
        using (var stream = File.OpenRead(path))
        using (var pe = new PEReader(stream))
        {
            var metadata = pe.GetMetadataReader();
            var directory = pe.PEHeaders.CorHeader.ResourcesDirectory;
            var section = pe.GetSectionData(directory.RelativeVirtualAddress).GetContent(0, directory.Size).ToArray();
            foreach (var handle in metadata.ManifestResources)
            {
                var resource = metadata.GetManifestResource(handle);
                if (!resource.Implementation.IsNil) continue;
                var offset = checked((int)resource.Offset);
                if (offset < 0 || offset > section.Length - 4) throw new InvalidDataException();
                var size = BitConverter.ToInt32(section, offset);
                if (size < 0 || size > section.Length - offset - 4) throw new InvalidDataException();
                Console.WriteLine("RESOURCE " + metadata.GetString(resource.Name));
                using (var bytes = new MemoryStream(section, offset + 4, size, false))
                using (var reader = new ResourceReader(bytes))
                {
                    var entries = reader.GetEnumerator();
                    var count = 0;
                    while (entries.MoveNext())
                    {
                        if (++count > 1024) throw new InvalidDataException();
                        var name = (string)entries.Key;
                        reader.GetResourceData(name, out var type, out var data);
                        Console.WriteLine(" ENTRY " + name + " " + type + " " + data.Length);
                        if (!name.Contains("graphics", StringComparison.OrdinalIgnoreCase)
                            || (type != "ResourceTypeCode.Stream" && type != "ResourceTypeCode.ByteArray")) continue;
                        var text = Encoding.UTF8.GetString(data);
                        var marker = Encoding.UTF8.GetBytes("cbLanguage");
                        for (var index = 0; index <= data.Length - marker.Length; index++)
                        {
                            if (!data.AsSpan(index, marker.Length).SequenceEqual(marker)) continue;
                            var length = Math.Min(320, data.Length - index);
                            Console.WriteLine(" LANGUAGE_HEX " + index + " " + Convert.ToHexString(data.AsSpan(index, length)));
                            break;
                        }
                        foreach (Match match in Regex.Matches(text, @"[\x20-\x7e]{2,128}"))
                            Console.WriteLine("  STRING " + match.Index + " " + match.Value);
                    }
                }
            }
        }
    }

    static string TokenName(MetadataReader reader, int token)
    {
        if ((token & unchecked((int)0xff000000)) == 0x70000000)
            return "\"" + reader.GetUserString(MetadataTokens.UserStringHandle(token & 0xffffff)) + "\"";
        var handle = MetadataTokens.EntityHandle(token);
        switch (handle.Kind)
        {
            case HandleKind.FieldDefinition: return reader.GetString(reader.GetFieldDefinition((FieldDefinitionHandle)handle).Name);
            case HandleKind.MethodDefinition: return reader.GetString(reader.GetMethodDefinition((MethodDefinitionHandle)handle).Name);
            case HandleKind.MemberReference: return reader.GetString(reader.GetMemberReference((MemberReferenceHandle)handle).Name);
            case HandleKind.TypeReference: return reader.GetString(reader.GetTypeReference((TypeReferenceHandle)handle).Name);
            case HandleKind.TypeDefinition: return reader.GetString(reader.GetTypeDefinition((TypeDefinitionHandle)handle).Name);
            default: return "token 0x" + token.ToString("x8");
        }
    }

    public static void Inspect(string path)
    {
        var opcodes = typeof(OpCodes).GetFields(BindingFlags.Public | BindingFlags.Static)
            .Where(f => f.FieldType == typeof(OpCode))
            .Select(f => (OpCode)f.GetValue(null)).ToDictionary(op => (ushort)op.Value);
        using (var stream = File.OpenRead(path))
        using (var pe = new PEReader(stream))
        {
            var reader = pe.GetMetadataReader();
            foreach (var typeHandle in reader.TypeDefinitions)
            {
                var type = reader.GetTypeDefinition(typeHandle);
                var typeName = reader.GetString(type.Name);
                if (reader.GetString(type.Namespace) != "ASMGameConfig" && typeName != "RegGoliath" && typeName != "GraphicsSettings") continue;
                Console.WriteLine("TYPE " + typeName);
                foreach (var fieldHandle in type.GetFields())
                {
                    var field = reader.GetFieldDefinition(fieldHandle);
                    var constantHandle = field.GetDefaultValue();
                    if (constantHandle.IsNil) continue;
                    var constant = reader.GetConstant(constantHandle);
                    var bytes = reader.GetBlobBytes(constant.Value);
                    Console.WriteLine(" CONSTANT " + reader.GetString(field.Name) + " " + constant.TypeCode + " " + Convert.ToHexString(bytes));
                }
                foreach (var methodHandle in type.GetMethods())
                {
                    var method = reader.GetMethodDefinition(methodHandle);
                    var name = reader.GetString(method.Name);
                    if (method.RelativeVirtualAddress == 0) continue;
                    if (typeName != "RegGoliath" && typeName != "GraphicsSettings" && !typeName.EndsWith("Preset")) continue;
                    Console.WriteLine(" METHOD " + name);
                    var body = pe.GetMethodBody(method.RelativeVirtualAddress).GetILBytes();
                    for (int offset = 0; offset < body.Length;)
                    {
                        int start = offset;
                        ushort value = body[offset++];
                        if (value == 0xfe) value = (ushort)(0xfe00 | body[offset++]);
                        var op = opcodes[value];
                        string operand = "";
                        int size;
                        switch (op.OperandType)
                        {
                            case OperandType.InlineNone: size = 0; break;
                            case OperandType.ShortInlineI: size = 1; operand = ((sbyte)body[offset]).ToString(); break;
                            case OperandType.ShortInlineVar: size = 1; operand = body[offset].ToString(); break;
                            case OperandType.InlineVar: size = 2; operand = BitConverter.ToUInt16(body, offset).ToString(); break;
                            case OperandType.ShortInlineBrTarget: size = 1; operand = "IL_" + (offset + 1 + (sbyte)body[offset]).ToString("x4"); break;
                            case OperandType.InlineBrTarget: size = 4; operand = "IL_" + (offset + 4 + BitConverter.ToInt32(body, offset)).ToString("x4"); break;
                            case OperandType.InlineI: size = 4; operand = BitConverter.ToInt32(body, offset).ToString(); break;
                            case OperandType.InlineI8: size = 8; operand = BitConverter.ToInt64(body, offset).ToString(); break;
                            case OperandType.ShortInlineR: size = 4; operand = BitConverter.ToSingle(body, offset).ToString(); break;
                            case OperandType.InlineR: size = 8; operand = BitConverter.ToDouble(body, offset).ToString(); break;
                            case OperandType.InlineSwitch: size = 4 + 4 * BitConverter.ToInt32(body, offset); operand = "switch"; break;
                            default: size = 4; operand = TokenName(reader, BitConverter.ToInt32(body, offset)); break;
                        }
                        Console.WriteLine("  IL_" + start.ToString("x4") + " " + op.Name + " " + operand);
                        offset = checked(offset + size);
                    }
                }
            }
        }
    }
}
'@

$fileHash | Format-List
$previousOutput = [Console]::Out
$capturedOutput = [System.IO.StringWriter]::new()
try {
    [Console]::SetOut($capturedOutput)
    if ($Resources) {
        [LauncherMetadataInspection]::InspectResources($resolvedPath)
    } else {
        [LauncherMetadataInspection]::Inspect($resolvedPath)
    }
} finally {
    [Console]::SetOut($previousOutput)
}
$capturedOutput.ToString()
$capturedOutput.Dispose()
