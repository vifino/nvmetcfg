(import ./lib.nix) {
  name = "nvmetcfg-test-rdma";
  nodes = {
    target = {
      self,
      pkgs,
      system,
      ...
    }: {
      environment.systemPackages = with pkgs; [
        self.packages.${system}.nvmetcfg-coverage
        llvmPackages_latest.bintools
      ];
      boot.kernelModules = ["nvmet" "nvmet_rdma" "rdma_rxe"];
      virtualisation.diskSize = 4096;
      networking.rxe.enable = true;
      networking.rxe.interfaces = ["eth1"];
      networking.firewall.allowedUDPPorts = [4791];
      environment.variables.LLVM_PROFILE_FILE = "/tmp/nvmetcfg-%p-%8m.profraw";
    };
    initiator = {
      self,
      pkgs,
      system,
      ...
    }: {
      environment.systemPackages = [pkgs.nvme-cli];
      boot.kernelModules = ["nvme_rdma" "rdma_rxe"];
      networking.rxe.enable = true;
      networking.rxe.interfaces = ["eth1"];
      networking.firewall.allowedUDPPorts = [4791];
    };
  };
  testScript = let
    subnqn = "nqn.2023-11.sh.tty:nvmetcfg-test-loop";
    initiator1 = "nqn.2024-02.sh.tty:nvmetcfg-client1";
    initiator2 = "nqn.2024-02.sh.tty:nvmetcfg-client2";
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

    target.succeed("nvmet subsystem add-host ${subnqn} ${initiator1}")
    target.succeed("test -d /sys/kernel/config/nvmet/hosts/${initiator1}")
    target.succeed("test -d /sys/kernel/config/nvmet/subsystems/${subnqn}/allowed_hosts/${initiator1}")
    assert "${initiator1}" in target.succeed("nvmet subsystem list-hosts ${subnqn}")
    target.succeed("nvmet subsystem add-host ${subnqn} ${initiator2}")
    target.succeed("test -d /sys/kernel/config/nvmet/hosts/${initiator2}")
    target.succeed("test -d /sys/kernel/config/nvmet/subsystems/${subnqn}/allowed_hosts/${initiator2}")
    assert "${initiator2}" in target.succeed("nvmet subsystem list-hosts ${subnqn}")
    target.succeed("nvmet subsystem remove-host ${subnqn} ${initiator1}")
    target.fail("test -e /sys/kernel/config/nvmet/hosts/${initiator1}")
    target.fail("test -e /sys/kernel/config/nvmet/subsystems/${subnqn}/allowed_hosts/${initiator1}")
    assert "${initiator1}" not in target.succeed("nvmet subsystem list-hosts ${subnqn}")

    target.succeed("nvmet subsystem show")

    target.succeed("nvmet namespace add ${subnqn} 1 /dev/loop0")
    assert "1" in target.succeed("nvmet namespace list ${subnqn}")
    target.succeed("test -d /sys/kernel/config/nvmet/subsystems/${subnqn}/namespaces/1")
    assert "/dev/loop0" in target.succeed("cat /sys/kernel/config/nvmet/subsystems/${subnqn}/namespaces/1/device_path")
    target.succeed("nvmet namespace show ${subnqn}")

    # Create the tcp port.
    target.succeed("nvmet port add 1 rdma 0.0.0.0:4420")
    assert "1" in target.succeed("nvmet port list")
    target.succeed("test -d /sys/kernel/config/nvmet/ports/1")
    assert "rdma" in target.succeed("cat /sys/kernel/config/nvmet/ports/1/addr_trtype")
    assert "ipv4" in target.succeed("cat /sys/kernel/config/nvmet/ports/1/addr_adrfam")
    assert "0.0.0.0" in target.succeed("cat /sys/kernel/config/nvmet/ports/1/addr_traddr")
    assert "4420" in target.succeed("cat /sys/kernel/config/nvmet/ports/1/addr_trsvcid")

    target.succeed("nvmet port add-subsystem 1 ${subnqn}")
    assert "${subnqn}" in target.succeed("nvmet port list-subsystems 1")
    target.succeed("test -h /sys/kernel/config/nvmet/ports/1/subsystems/${subnqn}")
    target.succeed("nvmet port show")

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
    clientnqn = initiator.succeed("nvme show-hostnqn")
    target.succeed("nvmet subsystem add-host ${subnqn} " + clientnqn)
    assert "${subnqn}" in initiator.succeed("nvme discover -t rdma -a target -s 4420 -q " + clientnqn)

    # Cleanup.
    target.succeed("nvmet namespace remove ${subnqn} 1")
    target.fail("test -e /sys/kernel/config/nvmet/subsystems/${subnqn}/namespaces/1")
    target.fail("nvmet namespace remove ${subnqn} 1")

    target.succeed("nvmet port remove-subsystem 1 ${subnqn}")
    assert "${subnqn}" not in target.succeed("nvmet port list-subsystems 1")
    target.fail("test -e /sys/kernel/config/nvmet/ports/1/subsystems/${subnqn}")

    target.succeed("nvmet subsystem remove ${subnqn}")
    target.fail("test -e /sys/kernel/config/nvmet/subsystems/${subnqn}")
    target.fail("nvmet subsystem remove ${subnqn}")

    target.succeed("nvmet port remove 1")
    target.fail("test -e /sys/kernel/config/nvmet/ports/1")
    target.fail("nvmet port remove 1")

    target.fail("test -e /sys/kernel/config/nvmet/hosts/${initiator2}")
    target.fail("test -e /sys/kernel/config/nvmet/subsystems/${subnqn}/allowed_hosts/${initiator2}")

    # Export coverage.
    target.succeed("llvm-profdata merge --sparse -o /tmp/nvmetcfg.profdata /tmp/nvmetcfg-*.profraw")
    target.succeed("llvm-cov export -format=lcov -instr-profile=/tmp/nvmetcfg.profdata " +
      "--ignore-filename-regex=/.cargo/registry --ignore-filename-regex=src/lib.rs --ignore-filename-regex=src/state/mod.rs " +
      "--show-instantiation-summary -object $(which nvmet) > /tmp/nvmet.lcov")
    target.copy_from_vm("/tmp/nvmet.lcov")
  '';
}
