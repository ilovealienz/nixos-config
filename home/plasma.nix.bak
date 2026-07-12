{ ... }:

{
  programs.plasma = {
    enable = true;
    kwin.titlebarButtons.right = [ "minimize" "maximize" "close" ];

    workspace = {
      colorScheme = "BreezeDark";
      iconTheme = "Papirus-Dark";
    };

    configFile."kwinrc"."org.kde.kdecoration2" = {
      library.value = "org.kde.breeze";
      theme.value = "Breeze";
      BorderSize.value = "Normal";
      BorderSizeAuto.value = false;
    };
    configFile."breezerc" = {
      "Common"."OutlineIntensity".value = "OutlineOff";
      "Common"."ShadowSize".value = "ShadowLarge";
      "Windeco"."DrawWinFocusLine".value = false;
    };
    configFile."kwinrc"."ElectricBorders" = {
      TopLeft.value = "None";
      Top.value = "None";
      TopRight.value = "None";
      Left.value = "None";
      Right.value = "None";
      BottomLeft.value = "None";
      Bottom.value = "None";
      BottomRight.value = "None";
    };
    configFile."ksmserverrc"."General" = {
      loginMode.value = "emptySession";
    };
  };
}
