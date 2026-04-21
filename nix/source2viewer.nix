{
  fetchFromGitHub,
  buildDotnetModule,
  dotnetCorePackages,
}:
# Builds ValveResourceFormat CLI from source
buildDotnetModule rec {
  pname = "ValveResourceFormat";
  version = "16.0";

  # GitHub source
  src = fetchFromGitHub {
    owner = "ValveResourceFormat";
    repo = "ValveResourceFormat";
    rev = "${version}";
    sha256 = "sha256-eFGCS0mS27z+3ffZ2no8XQ6znEm71vO9kR99SReVFdg=";
  };

  nugetDeps = ./deps.json;
  # .NET 10.0 runtime and SDK
  dotnet-sdk = dotnetCorePackages.sdk_10_0;
  dotnet-runtime = dotnetCorePackages.runtime_10_0;
  projectFile = "./CLI/CLI.csproj";
}
