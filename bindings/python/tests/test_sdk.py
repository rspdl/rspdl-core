import unittest

import rspdl


VALID_SOURCE = """@모듈 재고(inventory)

재고 항목(item)은 다음 필드들로 구성되어 있다.
    이름(name): 필수 문자열
"""


class SdkTest(unittest.TestCase):
    def test_compile_returns_versioned_workspace_result(self) -> None:
        response = rspdl.compile(
            [{"path": "inventory.rspdl", "text": VALID_SOURCE}]
        )

        self.assertEqual(response["schema_version"], 1)
        self.assertEqual(
            response["result"]["files"][0]["module"]["id"], "inventory"
        )

    def test_compiler_errors_remain_in_the_result(self) -> None:
        response = rspdl.compile([{"path": "invalid.rspdl", "text": "invalid"}])

        self.assertIsNone(response["result"]["files"][0]["module"])
        self.assertTrue(response["result"]["files"][0]["diagnostics"])

    def test_invalid_sdk_configuration_raises_a_stable_error(self) -> None:
        with self.assertRaisesRegex(RuntimeError, "RSPDL-SDK-003"):
            rspdl.compile(
                [{"path": "inventory.rspdl", "text": VALID_SOURCE}],
                locale="en-US",
            )

    def test_check_and_model_are_exposed(self) -> None:
        source = {"path": "inventory.rspdl", "text": VALID_SOURCE}

        checked = rspdl.check([source], {"records": {}})
        modeled = rspdl.find_model(source, scope_per_model=1)

        self.assertEqual(checked["schema_version"], 1)
        self.assertEqual(modeled["schema_version"], 1)


if __name__ == "__main__":
    unittest.main()
