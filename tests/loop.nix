(import ./lib.nix) {
  name = "nvmetcfg-test-loop";
  nodes.node = { self, pkgs, system, ... }: {
    environment.systemPackages = [ self.packages.${system}.default pkgs.nvme-cli ];
    boot.kernelModules = [ "nvmet" ];
    virtualisation.diskSize = 4096;
  };
  testScript = let 
    subnqn = "nqn.2023-11.sh.tty:nvmetcfg-test-loop";
  in ''
    start_all()
    node.wait_for_unit("default.target")

    # Set up the loop device.
    node.succeed("fallocate -l 1G /root/test.img")
    node.succeed("losetup /dev/loop0 /root/test.img")

    # Create our subsystems.
    node.succeed("nvmet subsystem add ${subnqn}")
    assert "${subnqn}" in node.succeed("nvmet subsystem list")
    node.succeed("test -d /sys/kernel/config/nvmet/subsystems/${subnqn}")

    node.succeed("nvmet namespace add ${subnqn} 1 /dev/loop0")
    assert "1" in node.succeed("nvmet namespace list ${subnqn}")
    node.succeed("test -d /sys/kernel/config/nvmet/subsystems/${subnqn}/namespaces/1")
    assert "/dev/loop0" in node.succeed("cat /sys/kernel/config/nvmet/subsystems/${subnqn}/namespaces/1/device_path")

    # Create the loopback port.
    node.succeed("nvmet port add 1 loop")
    assert "1" in node.succeed("nvmet port list")
    node.succeed("test -d /sys/kernel/config/nvmet/ports/1")
    assert "loop" in node.succeed("cat /sys/kernel/config/nvmet/ports/1/addr_trtype")

    node.succeed("nvmet port add-subsystem 1 ${subnqn}")
    assert "${subnqn}" in node.succeed("nvmet port list-subsystems 1")
    node.succeed("test -h /sys/kernel/config/nvmet/ports/1/subsystems/${subnqn}")
 
    assert "${subnqn}" in machine.succeed("nvme discover -t loop")

    # State save/restore test.
    node.succeed("nvmet state save /root/state.yml")
    node.succeed("test -f /root/state.yml")

    node.succeed("nvmet state clear")
    node.fail("test -e /sys/kernel/config/nvmet/subsystems/${subnqn}")
    node.fail("test -e /sys/kernel/config/nvmet/ports/1")

    node.succeed("nvmet state restore /root/state.yml")
    node.succeed("test -d /sys/kernel/config/nvmet/subsystems/${subnqn}/namespaces/1")
    node.succeed("test -d /sys/kernel/config/nvmet/ports/1")

    node.succeed("nvmet state save /root/state-after.yml")
    node.succeed("test -f /root/state-after.yml")
    assert node.succeed("cat /root/state.yml") == node.succeed("cat /root/state-after.yml")

    # Cleanup.
    node.succeed("nvmet subsystem remove ${subnqn}")
    node.fail("test -e /sys/kernel/config/nvmet/subsystems/${subnqn}")

    node.succeed("nvmet port remove 1")
    node.fail("test -e /sys/kernel/config/nvmet/ports/1")
  '';
}
