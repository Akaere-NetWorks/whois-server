using WhoisServer.Core.Patch;
using Xunit;

namespace WhoisServer.Core.Tests;

/// <summary>Ported from src/core/patch.rs's #[cfg(test)] module, plus extra coverage for the
/// riskiest behaviors called out in plan.md §7.1 (source: block scoping, context rules).</summary>
public class PatchTests
{
    private static PatchFile SinglePatchFile(PatchRule patch) => new("test.patch", [patch]);

    [Fact]
    public void SimpleReplacement()
    {
        var patchFile = SinglePatchFile(new PatchRule(
            Conditions: [],
            Excludes: [],
            ContextRules: [],
            Hunks: [new DiffHunk(["RuiNetwork"], ["Ruifeng Enterprise"])]));

        var result = PatchApplier.ApplyPatches("query", "netname: RuiNetwork", [patchFile]);

        Assert.Equal("netname: Ruifeng Enterprise", result);
    }

    [Fact]
    public void QueryContainsCondition_GatesWhetherThePatchApplies()
    {
        var patchFile = SinglePatchFile(new PatchRule(
            Conditions: [new PatchCondition(ConditionType.QueryContains, "RuiNetwork", null)],
            Excludes: [],
            ContextRules: [],
            Hunks: [new DiffHunk(["RuiNetwork"], ["Ruifeng Enterprise"])]));

        var matched = PatchApplier.ApplyPatches("AS-RuiNetwork", "netname: RuiNetwork", [patchFile]);
        Assert.Equal("netname: Ruifeng Enterprise", matched);

        var unmatched = PatchApplier.ApplyPatches("AS12345", "netname: RuiNetwork", [patchFile]);
        Assert.Equal("netname: RuiNetwork", unmatched);
    }

    [Fact]
    public void ResponseContainsCondition_GatesWhetherThePatchApplies()
    {
        var patchFile = SinglePatchFile(new PatchRule(
            Conditions: [new PatchCondition(ConditionType.ResponseContains, "RuiNetwork", null)],
            Excludes: [],
            ContextRules: [],
            Hunks: [new DiffHunk(["RuiNetwork"], ["Ruifeng Enterprise"])]));

        var matched = PatchApplier.ApplyPatches("", "netname: RuiNetwork", [patchFile]);
        Assert.Equal("netname: Ruifeng Enterprise", matched);

        var unmatched = PatchApplier.ApplyPatches("", "netname: Other", [patchFile]);
        Assert.Equal("netname: Other", unmatched);
    }

    [Fact]
    public void SourceFieldReplacement_OnlyAppliesInsideUserEditableObjectBlocks()
    {
        var patchFile = SinglePatchFile(new PatchRule(
            Conditions: [],
            Excludes: [],
            ContextRules: [],
            Hunks: [new DiffHunk(["^source:"], ["source:         REWRITTEN"])]));

        // Inside an "aut-num:" block -> replace.
        var autNumResponse = "aut-num:        AS64512\nsource:         DN42\n";
        var autNumResult = PatchApplier.ApplyPatches("q", autNumResponse, [patchFile]);
        Assert.Contains("source:         REWRITTEN", autNumResult);

        // Inside an "inetnum:" (registry-authoritative) block -> must NOT replace.
        var inetnumResponse = "inetnum:        10.0.0.0 - 10.0.0.255\nsource:         DN42\n";
        var inetnumResult = PatchApplier.ApplyPatches("q", inetnumResponse, [patchFile]);
        Assert.Contains("source:         DN42", inetnumResult);
        Assert.DoesNotContain("REWRITTEN", inetnumResult);
    }

    [Fact]
    public void SkipAfterContextRule_SuppressesReplacementWhenPatternFollows()
    {
        var patchFile = SinglePatchFile(new PatchRule(
            Conditions: [],
            Excludes: [],
            ContextRules: [new ContextRule("DO-NOT-TOUCH", ContextDirection.After, 3, ContextAction.Skip)],
            Hunks: [new DiffHunk(["FooBar"], ["Replaced"])]));

        var response = "FooBar\nline1\nDO-NOT-TOUCH\n";
        var result = PatchApplier.ApplyPatches("q", response, [patchFile]);

        Assert.Contains("FooBar", result);
        Assert.DoesNotContain("Replaced", result);
    }

    [Fact]
    public void OnlyAfterContextRule_RequiresPatternWithinWindow()
    {
        var patchFile = SinglePatchFile(new PatchRule(
            Conditions: [],
            Excludes: [],
            ContextRules: [new ContextRule("ALLOW-ME", ContextDirection.After, 2, ContextAction.Only)],
            Hunks: [new DiffHunk(["FooBar"], ["Replaced"])]));

        var withinWindow = "FooBar\nALLOW-ME\n";
        Assert.Contains("Replaced", PatchApplier.ApplyPatches("q", withinWindow, [patchFile]));

        var outsideWindow = "FooBar\nx\nx\nx\nALLOW-ME\n";
        Assert.DoesNotContain("Replaced", PatchApplier.ApplyPatches("q", outsideWindow, [patchFile]));
    }

    [Fact]
    public void ExcludePattern_SkipsMatchingLines()
    {
        var patchFile = SinglePatchFile(new PatchRule(
            Conditions: [],
            Excludes: ["descr:"],
            ContextRules: [],
            Hunks: [new DiffHunk(["FooBar"], ["Replaced"])]));

        var response = "descr: FooBar\nnetname: FooBar\n";
        var result = PatchApplier.ApplyPatches("q", response, [patchFile]);

        Assert.Contains("descr: FooBar", result);
        Assert.Contains("netname: Replaced", result);
    }

    [Fact]
    public void ParsesRealPatchDsl()
    {
        var content = """
            # QUERY_CONTAINS: RuiNetwork
            ---
            +++
            @@ -1 +1 @@
            -RuiNetwork
            +Ruifeng Enterprise
            """;

        var patchFile = PatchParser.ParsePatchContent("001-example.patch", content);

        Assert.Single(patchFile.Patches);
        var patch = patchFile.Patches[0];
        Assert.Single(patch.Conditions);
        Assert.Equal(ConditionType.QueryContains, patch.Conditions[0].ConditionType);
        Assert.Equal("RuiNetwork", patch.Conditions[0].Value);
        Assert.Single(patch.Hunks);
        Assert.Equal(["RuiNetwork"], patch.Hunks[0].RemoveLines);
        Assert.Equal(["Ruifeng Enterprise"], patch.Hunks[0].AddLines);

        var result = PatchApplier.ApplyPatches("AS-RuiNetwork", "netname: RuiNetwork", [patchFile]);
        Assert.Equal("netname: Ruifeng Enterprise", result);
    }
}
