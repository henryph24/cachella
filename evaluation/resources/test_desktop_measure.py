import os
import platform
from pathlib import Path
import unittest
from desktop_measure import select_members, validate_coalition, verify_members, collect


def row(pid, parent, coalition, name='fixture'):
    return {'pid': pid, 'parent_pid': parent, 'coalition': coalition, 'executable': name}


class MembershipTests(unittest.TestCase):
    def test_includes_reparented_xpc_and_descendants_but_excludes_other_apps(self):
        rows = [row(10, 1, [100, 101]), row(20, 1, [100, 101]),
                row(30, 10, [200, 201]), row(40, 30, [200, 201]),
                row(50, 1, [100, 999]), row(60, 1, [999, 101])]
        result = select_members(rows, [100, 101])
        self.assertEqual([p['pid'] for p in result], [10, 20, 30, 40])

    def test_root_exit_still_includes_lingering_coalition_helpers(self):
        self.assertEqual([p['pid'] for p in select_members(
            [row(20, 1, [100, 101]), row(30, 1, [200, 201])], [100, 101])], [20])

    def test_reused_root_pid_does_not_adopt_new_process_tree(self):
        rows = [row(10, 1, [200, 201]), row(30, 10, [200, 201]), row(20, 1, [100, 101])]
        self.assertEqual([p['pid'] for p in select_members(rows, [100, 101])], [20])

    def test_replaced_or_exited_member_invalidates_accounting(self):
        class Reader:
            def read(self, pid):
                if pid == 20:
                    raise OSError(3, 'exited')
                return [200, 201]
        result = verify_members(Reader(), [row(10, 1, [100, 101]), row(20, 1, [100, 101])])
        self.assertTrue(all('error' in p for p in result))

    @unittest.skipUnless(platform.system() == 'Darwin', 'macOS coalition ABI')
    def test_rejects_a_target_in_the_collectors_own_coalition(self):
        with self.assertRaisesRegex(ValueError, 'collector shares its coalition'):
            collect(os.getpid(), Path('unused'), Path('unused'), Path('unused'), 1, .01)

    def test_unavailable_or_zero_coalition_fails_closed(self):
        for value in [None, [], [0, 0], [0, 1], [1], [1, 2, 3]]:
            with self.assertRaises(ValueError):
                validate_coalition(value)
        self.assertEqual(validate_coalition([100, 101]), [100, 101])


if __name__ == '__main__':
    unittest.main()
