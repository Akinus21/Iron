class Iron < Formula
  desc "GTK4 keyboard-driven web browser for BlueAK"
  homepage "https://github.com/Akinus21/Iron"
  version "{{VERSION}}"
  url "{{URL}}"
  sha256 "{{SHA256}}"

  depends_on "gtk4"
  depends_on "libadwaita"

  resource "cef-runtime" do
    url "{{CEF_RUNTIME_URL}}"
    sha256 "{{CEF_RUNTIME_SHA256}}"
  end

  def install
    bin.install "iron" => "iron.bin"

    resource("cef-runtime").stage do
      lib.install "libcef.so"
      Dir.glob("*.so").reject { |f| f == "libcef.so" }.each { |f| lib.install f }
      Dir.glob("*.so.*").each { |f| lib.install f }
      Dir.glob("*.dat").each { |f| lib.install f }
      Dir.glob("*.bin").each { |f| lib.install f }
      Dir.glob("*.json").each { |f| lib.install f }
      Dir.glob("*.pak").each { |f| lib.install f }
      (lib/"swiftshader").install Dir.glob("swiftshader/*") if Dir.exist?("swiftshader")
      (share/"iron").install Dir.glob("*.pak")
      (share/"iron"/"locales").install Dir.glob("locales/*") if Dir.exist?("locales")
    end

    (bin/"iron").write <<~SH
      #!/bin/bash
      export LD_LIBRARY_PATH="#{lib}${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
      exec "#{bin}/iron.bin" "$@"
    SH
  end

  test do
    assert_match "iron", shell_output("#{bin}/iron --version 2>&1 || true")
  end
end
