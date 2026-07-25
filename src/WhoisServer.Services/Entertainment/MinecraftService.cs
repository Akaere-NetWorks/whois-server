using System.Net.Http.Json;
using System.Net.Sockets;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace WhoisServer.Services.Entertainment;

/// <summary>
/// Ported from src/services/minecraft.rs — the <c>-MINECRAFT</c>/<c>-MC</c> suffixes (server status,
/// via the raw Minecraft Server List Ping protocol over TCP — there is no third-party HTTP status
/// API involved) and the <c>-MCU</c> suffix (player lookup via Mojang's public HTTP APIs).
/// </summary>
public sealed class MinecraftService(HttpClient httpClient)
{
    private static readonly TimeSpan ConnectTimeout = TimeSpan.FromSeconds(10);
    private const int DefaultPort = 25565;
    private const int ProtocolVersion = 764; // recent protocol number; status ping ignores mismatches

    // ── -MINECRAFT / -MC: server status via the Server List Ping protocol ────────────────────

    /// <summary><paramref name="baseQuery"/> is already suffix-stripped. Target format is
    /// <c>hostname:port</c> or bare <c>hostname</c> (defaults to port 25565).</summary>
    public async Task<string> ProcessServerAsync(string baseQuery, CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(baseQuery))
        {
            return "Invalid Minecraft query. Target format: hostname:port or hostname (default port 25565)\n" +
                   "Examples:\n  - mc.hypixel.net-MC\n  - play.cubecraft.net:25565-MINECRAFT\n";
        }

        string host;
        int port;
        try
        {
            (host, port) = ParseTarget(baseQuery);
        }
        catch (Exception e)
        {
            return $"Invalid Minecraft target '{baseQuery}': {e.Message}\n";
        }

        try
        {
            var info = await QueryServerStatusAsync(host, port, ct);
            return FormatServerInfo(info);
        }
        catch (Exception e)
        {
            return $"""
                Minecraft Server Query Failed for {host}:{port}
                Error: {e.Message}

                Possible causes:
                - Server is offline or unreachable
                - Server is not running Minecraft
                - Firewall blocking connection
                - Invalid hostname or port

                """;
        }
    }

    private static (string Host, int Port) ParseTarget(string target)
    {
        var colonPos = target.LastIndexOf(':');
        if (colonPos < 0)
        {
            return (target, DefaultPort);
        }

        var host = target[..colonPos];
        var portStr = target[(colonPos + 1)..];
        if (host.Length == 0)
        {
            throw new FormatException("Empty hostname");
        }

        if (!int.TryParse(portStr, out var port) || port is <= 0 or > 65535)
        {
            throw new FormatException($"Invalid port number: {portStr}");
        }

        return (host, port);
    }

    private static async Task<MinecraftServerInfo> QueryServerStatusAsync(string host, int port, CancellationToken ct)
    {
        var stopwatch = System.Diagnostics.Stopwatch.StartNew();

        using var tcpClient = new TcpClient();
        using var timeoutCts = CancellationTokenSource.CreateLinkedTokenSource(ct);
        timeoutCts.CancelAfter(ConnectTimeout);

        await tcpClient.ConnectAsync(host, port, timeoutCts.Token);
        await using var stream = tcpClient.GetStream();

        // Handshake (packet 0x00): protocol version, server address, server port, next-state=1 (status)
        var handshake = new List<byte> { 0x00 };
        WriteVarInt(handshake, ProtocolVersion);
        WriteString(handshake, host);
        handshake.Add((byte)((port >> 8) & 0xFF));
        handshake.Add((byte)(port & 0xFF));
        WriteVarInt(handshake, 1);
        await SendPacketAsync(stream, handshake, timeoutCts.Token);

        // Status request (packet 0x00, empty body)
        await SendPacketAsync(stream, [0x00], timeoutCts.Token);

        var statusJson = await ReadStatusResponseAsync(stream, timeoutCts.Token);
        var status = JsonSerializer.Deserialize<MinecraftStatus>(statusJson)
            ?? throw new InvalidOperationException("Failed to parse server status JSON");

        // Ping (packet 0x01 + 8-byte payload) for latency measurement
        var pingStart = stopwatch.Elapsed;
        var pingPacket = new List<byte> { 0x01 };
        var timestamp = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
        for (var i = 7; i >= 0; i--)
        {
            pingPacket.Add((byte)((timestamp >> (i * 8)) & 0xFF));
        }

        await SendPacketAsync(stream, pingPacket, timeoutCts.Token);
        await ReadPacketAsync(stream, timeoutCts.Token); // pong response, payload not needed
        var pingLatency = (long)(stopwatch.Elapsed - pingStart).TotalMilliseconds;

        var playerList = (status.Players.Sample ?? []).Select(p => p.Name).ToList();
        var description = FormatDescription(status.Description);

        return new MinecraftServerInfo(
            host, port, true, status.Version.Name, status.Version.Protocol,
            status.Players.Online, status.Players.Max, playerList, description,
            pingLatency, status.EnforcesSecureChat, status.PreviewsChat);
    }

    private static async Task<string> ReadStatusResponseAsync(NetworkStream stream, CancellationToken ct)
    {
        var packet = await ReadPacketAsync(stream, ct);
        if (packet.Length == 0 || packet[0] != 0x00)
        {
            throw new InvalidOperationException("Invalid status response packet");
        }

        var offset = 1;
        var length = ReadVarIntFromBytes(packet, ref offset);
        if (offset + length > packet.Length)
        {
            throw new InvalidOperationException("String length exceeds available data");
        }

        return Encoding.UTF8.GetString(packet, offset, length);
    }

    private static string FormatDescription(JsonElement description)
    {
        return description.ValueKind switch
        {
            JsonValueKind.String => description.GetString() ?? "",
            JsonValueKind.Object when description.TryGetProperty("text", out var text) && text.ValueKind == JsonValueKind.String
                => text.GetString() ?? "",
            _ => description.ToString(),
        };
    }

    private static string FormatServerInfo(MinecraftServerInfo info)
    {
        var sb = new StringBuilder();
        sb.Append("% This is the WHOIS server response for Minecraft server query\n");
        sb.Append("% Information related to Minecraft server status\n");
        sb.Append("%\n% The objects are in RPSL format\n%\n");

        sb.Append($"server:         {info.Address}:{info.Port}\n");
        sb.Append($"status:         {(info.Online ? "ONLINE" : "OFFLINE")}\n");
        sb.Append($"version:        {info.Version}\n");
        sb.Append($"protocol:       {info.Protocol}\n");
        sb.Append($"descr:          {info.Description}\n");
        sb.Append($"players-online: {info.PlayersOnline}\n");
        sb.Append($"players-max:    {info.PlayersMax}\n");
        sb.Append($"latency:        {info.Latency}ms\n");

        if (info.EnforcesSecureChat is { } secureChat)
        {
            sb.Append($"secure-chat:    {(secureChat ? "enforced" : "optional")}\n");
        }

        if (info.PreviewsChat is { } previewChat)
        {
            sb.Append($"chat-preview:   {(previewChat ? "enabled" : "disabled")}\n");
        }

        if (info.PlayerList.Count > 0)
        {
            for (var i = 0; i < info.PlayerList.Count; i++)
            {
                if (i >= 10)
                {
                    sb.Append($"remarks:        ... and {info.PlayerList.Count - 10} more players online\n");
                    break;
                }

                sb.Append($"player:         {info.PlayerList[i]}\n");
            }
        }
        else if (info.PlayersOnline > 0)
        {
            sb.Append("remarks:        Player list hidden by server configuration\n");
        }

        sb.Append("source:         AKAERE-NETWORKS-AGENT\n\n");
        sb.Append("% Information retrieved using Minecraft Server List Ping protocol\n");
        sb.Append("% Query processed by WHOIS server\n");
        return sb.ToString();
    }

    // ── VarInt / packet framing helpers ────────────────────────────────────────────────────────

    private static void WriteVarInt(List<byte> buffer, int value)
    {
        var v = unchecked((uint)value);
        while (true)
        {
            var b = (byte)(v & 0x7F);
            v >>= 7;
            if (v != 0)
            {
                b |= 0x80;
            }

            buffer.Add(b);
            if (v == 0)
            {
                break;
            }
        }
    }

    private static void WriteString(List<byte> buffer, string value)
    {
        var bytes = Encoding.UTF8.GetBytes(value);
        WriteVarInt(buffer, bytes.Length);
        buffer.AddRange(bytes);
    }

    private static async Task SendPacketAsync(NetworkStream stream, List<byte> payload, CancellationToken ct)
    {
        var framed = new List<byte>();
        WriteVarInt(framed, payload.Count);
        framed.AddRange(payload);
        await stream.WriteAsync(framed.ToArray(), ct);
    }

    private static async Task<byte[]> ReadPacketAsync(NetworkStream stream, CancellationToken ct)
    {
        var length = await ReadVarIntAsync(stream, ct);
        if (length == 0)
        {
            return [];
        }

        if (length > 1_048_576)
        {
            throw new InvalidOperationException($"Packet too large: {length} bytes");
        }

        var buffer = new byte[length];
        var offset = 0;
        while (offset < length)
        {
            var read = await stream.ReadAsync(buffer.AsMemory(offset, length - offset), ct);
            if (read == 0)
            {
                throw new EndOfStreamException("Connection closed while reading packet data");
            }

            offset += read;
        }

        return buffer;
    }

    private static async Task<int> ReadVarIntAsync(NetworkStream stream, CancellationToken ct)
    {
        var result = 0;
        var position = 0;
        var buffer = new byte[1];

        while (true)
        {
            var read = await stream.ReadAsync(buffer.AsMemory(0, 1), ct);
            if (read == 0)
            {
                throw new EndOfStreamException("Connection closed while reading VarInt");
            }

            var b = buffer[0];
            result |= (b & 0x7F) << position;
            if ((b & 0x80) == 0)
            {
                break;
            }

            position += 7;
            if (position >= 32)
            {
                throw new InvalidOperationException("VarInt too big");
            }
        }

        return result;
    }

    private static int ReadVarIntFromBytes(byte[] data, ref int offset)
    {
        var result = 0;
        var position = 0;

        while (true)
        {
            if (offset >= data.Length)
            {
                throw new EndOfStreamException("Unexpected end of data while reading varint");
            }

            var b = data[offset++];
            result |= (b & 0x7F) << position;
            if ((b & 0x80) == 0)
            {
                break;
            }

            position += 7;
            if (position >= 32)
            {
                throw new InvalidOperationException("VarInt too big");
            }
        }

        return result;
    }

    // ── -MCU: player lookup via Mojang's public APIs ──────────────────────────────────────────

    /// <summary><paramref name="baseQuery"/> is already suffix-stripped and expected to be a
    /// Minecraft username (3-16 alphanumeric/underscore characters).</summary>
    public async Task<string> ProcessUserAsync(string baseQuery, CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(baseQuery))
        {
            return "Invalid Minecraft user query. Please provide a username.\nExample: Notch-MCU\n";
        }

        if (baseQuery.Length is < 3 or > 16 || !baseQuery.All(c => char.IsLetterOrDigit(c) || c == '_'))
        {
            return $"Invalid Minecraft username format. Usernames must be 3-16 characters, alphanumeric and underscores only.\nQuery: {baseQuery}\n";
        }

        MojangUuidResponse uuid;
        try
        {
            uuid = await GetUuidFromUsernameAsync(baseQuery, ct);
        }
        catch (Exception e)
        {
            return $"Minecraft User Not Found: {baseQuery}\nError: {e.Message}\n";
        }

        try
        {
            var profile = await GetUserProfileAsync(uuid.Id, ct);
            return FormatUserInfo(uuid, profile);
        }
        catch (Exception e)
        {
            return FormatBasicUserInfo(uuid, $"Profile fetch failed: {e.Message}");
        }
    }

    private async Task<MojangUuidResponse> GetUuidFromUsernameAsync(string username, CancellationToken ct)
    {
        var url = $"https://api.mojang.com/users/profiles/minecraft/{Uri.EscapeDataString(username)}";
        using var response = await httpClient.GetAsync(url, ct);

        if (response.StatusCode == System.Net.HttpStatusCode.NoContent)
        {
            throw new InvalidOperationException("Player not found");
        }

        if (!response.IsSuccessStatusCode)
        {
            throw new InvalidOperationException($"API request failed: {(int)response.StatusCode}");
        }

        return await response.Content.ReadFromJsonAsync<MojangUuidResponse>(cancellationToken: ct)
            ?? throw new InvalidOperationException("Empty response");
    }

    private async Task<MojangProfileResponse> GetUserProfileAsync(string uuid, CancellationToken ct)
    {
        var url = $"https://sessionserver.mojang.com/session/minecraft/profile/{uuid}";
        using var response = await httpClient.GetAsync(url, ct);
        if (!response.IsSuccessStatusCode)
        {
            throw new InvalidOperationException($"Profile request failed: {(int)response.StatusCode}");
        }

        return await response.Content.ReadFromJsonAsync<MojangProfileResponse>(cancellationToken: ct)
            ?? throw new InvalidOperationException("Empty response");
    }

    private static string FormatUserInfo(MojangUuidResponse uuid, MojangProfileResponse profile)
    {
        var sb = new StringBuilder();
        sb.Append($"Minecraft User Information: {uuid.Name}\n");
        sb.Append(new string('=', 60)).Append('\n');
        sb.Append($"username: {uuid.Name}\n");
        sb.Append($"uuid: {FormatUuid(uuid.Id)}\n");
        sb.Append($"uuid-short: {uuid.Id}\n");

        foreach (var property in profile.Properties ?? [])
        {
            if (property.Name == "textures")
            {
                sb.Append("has-skin: yes\n");
                if (property.Signature is not null)
                {
                    sb.Append("skin-signed: yes\n");
                }
            }
            else
            {
                sb.Append($"property-{property.Name.ToLowerInvariant()}: present\n");
            }
        }

        sb.Append($"namemc-url: https://namemc.com/profile/{uuid.Id}\n");
        sb.Append($"skin-url: https://crafatar.com/skins/{uuid.Id}\n");
        sb.Append($"avatar-url: https://crafatar.com/avatars/{uuid.Id}\n");
        sb.Append("% Note: Player information is public data from Mojang API\n");
        return sb.ToString();
    }

    private static string FormatBasicUserInfo(MojangUuidResponse uuid, string status)
    {
        var sb = new StringBuilder();
        sb.Append($"Minecraft User Information: {uuid.Name}\n");
        sb.Append(new string('=', 60)).Append('\n');
        sb.Append($"username: {uuid.Name}\n");
        sb.Append($"uuid: {FormatUuid(uuid.Id)}\n");
        sb.Append($"uuid-short: {uuid.Id}\n");
        sb.Append($"profile-status: {status}\n");
        sb.Append($"namemc-url: https://namemc.com/profile/{uuid.Id}\n");
        sb.Append($"skin-url: https://crafatar.com/skins/{uuid.Id}\n");
        sb.Append($"avatar-url: https://crafatar.com/avatars/{uuid.Id}\n");
        return sb.ToString();
    }

    private static string FormatUuid(string uuid) =>
        uuid.Length == 32
            ? $"{uuid[..8]}-{uuid[8..12]}-{uuid[12..16]}-{uuid[16..20]}-{uuid[20..32]}"
            : uuid;

    private sealed record MinecraftServerInfo(
        string Address, int Port, bool Online, string Version, int Protocol,
        int PlayersOnline, int PlayersMax, List<string> PlayerList, string Description,
        long Latency, bool? EnforcesSecureChat, bool? PreviewsChat);

    private sealed record MinecraftStatus(
        [property: JsonPropertyName("version")] MinecraftVersion Version,
        [property: JsonPropertyName("players")] MinecraftPlayers Players,
        [property: JsonPropertyName("description")] JsonElement Description,
        [property: JsonPropertyName("enforcesSecureChat")] bool? EnforcesSecureChat,
        [property: JsonPropertyName("previewsChat")] bool? PreviewsChat);

    private sealed record MinecraftVersion(
        [property: JsonPropertyName("name")] string Name,
        [property: JsonPropertyName("protocol")] int Protocol);

    private sealed record MinecraftPlayers(
        [property: JsonPropertyName("max")] int Max,
        [property: JsonPropertyName("online")] int Online,
        [property: JsonPropertyName("sample")] List<MinecraftPlayerSample>? Sample);

    private sealed record MinecraftPlayerSample([property: JsonPropertyName("name")] string Name);

    private sealed record MojangUuidResponse(
        [property: JsonPropertyName("id")] string Id,
        [property: JsonPropertyName("name")] string Name);

    private sealed record MojangProfileResponse(
        [property: JsonPropertyName("id")] string Id,
        [property: JsonPropertyName("name")] string Name,
        [property: JsonPropertyName("properties")] List<MojangProfileProperty>? Properties);

    private sealed record MojangProfileProperty(
        [property: JsonPropertyName("name")] string Name,
        [property: JsonPropertyName("value")] string Value,
        [property: JsonPropertyName("signature")] string? Signature);
}
