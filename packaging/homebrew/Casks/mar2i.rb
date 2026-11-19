cask "mar2i" do
  arch arm: "aarch64", intel: "x64"

  version "0.2.3"
  sha256 arm:   "d78ec65b9ec47a3ad33788a4d1540ba0ffc5c2fb1db6710d50d0d934a7c31806",
         intel: "d78ec65b9ec47a3ad33788a4d1540ba0ffc5c2fb1db6710d50d0d934a7c31806"

  url "https://github.com/imMamdouhaboammar/mar2i/releases/download/v#{version}/Mar2i_#{version}_#{arch}.dmg",
      verified: "github.com/imMamdouhaboammar/mar2i/"
  name "Mar2i"
  name "مرئي"
  desc "Offline bulk video compression and archival desktop application"
  homepage "https://github.com/imMamdouhaboammar/mar2i"

  livecheck do
    url :url
    strategy :github_latest
  end

  app "Mar2i.app"

  zap trash: [
    "~/Library/Application Support/com.mamdouh.mar2i",
    "~/Library/Caches/com.mamdouh.mar2i",
    "~/Library/Preferences/com.mamdouh.mar2i.plist",
    "~/Library/Saved Application State/com.mamdouh.mar2i.savedState",
    "~/Library/WebKit/com.mamdouh.mar2i",
  ]

  caveats <<~EOS
    Mar2i is distributed with ad-hoc signing. To bypass macOS Gatekeeper
    quarantine verification during Homebrew installation, use:

      brew install --cask --no-quarantine imMamdouhaboammar/mar2i/mar2i

    If you installed without --no-quarantine and macOS prevents the app from opening,
    remove the quarantine attribute manually by running:

      xattr -cr /Applications/Mar2i.app
  EOS
end
