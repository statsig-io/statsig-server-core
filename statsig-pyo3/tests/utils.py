import os


def get_test_data_resource(filename: str) -> str:
    root = os.path.dirname(os.path.abspath(__file__))
    with open(
        os.path.join(root, "../../statsig-lib/tests/data", filename), "r"
    ) as file:
        file_content = file.read()

    return file_content
