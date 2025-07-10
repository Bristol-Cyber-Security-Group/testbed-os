import os, importlib

# the following automatically imports the scripts for each test case so that the self-registering works on load
local_package_path = os.path.dirname(__file__)
for filename in os.listdir(local_package_path):
    if filename.endswith(".py") and filename not in ["__init__.py", "shared_runtime_tests.py", "test_case.py"]:
        module_name = f".{filename[:-3]}"
        importlib.import_module(module_name, __name__)
