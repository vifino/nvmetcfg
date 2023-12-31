(import ./lib.nix) {
  name = "nvmetcfg-test-tcp";
  nodes = {
    target = { self, pkgs, system, ... }: {
      environment.systemPackages = [ self.packages.${system}.default ];
      boot.kernelModules = [ "nvmet" "nvmet_tcp" ];
      virtualisation.diskSize = 4096;
      networking.firewall.allowedTCPPorts = [ 4420 ];
    };
    initiator = { self, pkgs, system, ... }: {
      environment.systemPackages = [ pkgs.nvme-cli ];
      boot.kernelModules = [ "nvme_tcp" ];
    };
  };
  testScript = let 
    subnqn = "nqn.2023-11.sh.tty:nvmetcfg-test-loop";
  in ''
    start_all()
    target.wait_for_unit("default.target")

    # Set up the loop device.
    target.succeed("fallocate -l 1G /root/test.img")
    target.succeed("losetup /dev/loop0 /root/test.img")

    # Create our subsystems.
    target.succeed("nvmet subsystem add ${subnqn}")
    assert "${subnqn}" in target.succeed("nvmet subsystem list")
    target.succeed("test -d /sys/kernel/config/nvmet/subsystems/${subnqn}")

    target.succeed("nvmet namespace add ${subnqn} 1 /dev/loop0")
    assert "1" in target.succeed("nvmet namespace list ${subnqn}")
    target.succeed("test -d /sys/kernel/config/nvmet/subsystems/${subnqn}/namespaces/1")
    assert "/dev/loop0" in target.succeed("cat /sys/kernel/config/nvmet/subsystems/${subnqn}/namespaces/1/device_path")

    # Create the tcp port.
    target.succeed("nvmet port add 1 tcp 0.0.0.0:4420")
    assert "1" in target.succeed("nvmet port list")
    target.succeed("test -d /sys/kernel/config/nvmet/ports/1")
    assert "tcp" in target.succeed("cat /sys/kernel/config/nvmet/ports/1/addr_trtype")
    assert "ipv4" in target.succeed("cat /sys/kernel/config/nvmet/ports/1/addr_adrfam")
    assert "0.0.0.0" in target.succeed("cat /sys/kernel/config/nvmet/ports/1/addr_traddr")
    assert "4420" in target.succeed("cat /sys/kernel/config/nvmet/ports/1/addr_trsvcid")

    target.succeed("nvmet port add-subsystem 1 ${subnqn}")
    assert "${subnqn}" in target.succeed("nvmet port list-subsystems 1")
    target.succeed("test -h /sys/kernel/config/nvmet/ports/1/subsystems/${subnqn}")
 
    # State save/restore test.
    target.succeed("nvmet state save /root/state.yml")
    target.succeed("test -f /root/state.yml")

    target.succeed("nvmet state clear")
    target.fail("test -e /sys/kernel/config/nvmet/subsystems/${subnqn}")
    target.fail("test -e /sys/kernel/config/nvmet/ports/1")

    target.succeed("nvmet state restore /root/state.yml")
    target.succeed("test -d /sys/kernel/config/nvmet/subsystems/${subnqn}/namespaces/1")
    target.succeed("test -d /sys/kernel/config/nvmet/ports/1")

    target.succeed("nvmet state save /root/state-after.yml")
    target.succeed("test -f /root/state-after.yml")
    assert target.succeed("cat /root/state.yml") == target.succeed("cat /root/state-after.yml")

    # Test the target on the initiator.
    initiator.wait_for_unit("default.target")
    assert "${subnqn}" in initiator.succeed("nvme discover -t tcp -a target -s 4420")

    # Cleanup.
    target.succeed("nvmet subsystem remove ${subnqn}")
    target.fail("test -e /sys/kernel/config/nvmet/subsystems/${subnqn}")

    target.succeed("nvmet port remove 1")
    target.fail("test -e /sys/kernel/config/nvmet/ports/1")
  '';
}
