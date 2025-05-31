# Test Cases

This folder contains the test cases for the test-harness.

To add a new test case, you must do two things:

1. create a named test case folder, containing the `kvm-compose.yaml` file, and any other artefacts that will be used by the guests
2. create a named test case python script, that will describe how the test is run

In the python script, you must do the following:

1. Create a class that inherits from `TestCase` 
2. Implement the function `test_case`.
2. Register this class into the `registered_test_cases` list

Example:

```python
import logging
import time
from typing import List
from host import LinkedCloneHost
from .test_case import TestCase, registered_test_cases

class BaseTestCase(TestCase):  # <--------------- 1.

    def __init__(self, linked_clone_hosts: List[LinkedCloneHost]):
        super().__init__("base", linked_clone_hosts)

    def test_case(self) -> List[TestReport]:  # <------------ 2.
        
        # Here you implement tests inside the guests defined in the yaml
        
        test_1 = ...  # This MUST be of type ``TestReport``
        test_2 = ...
        
        return [test_1, test_2]  # return all tests in a list

registered_test_cases.append(BaseTestCase)  # <-- 3.
```

To implement tests, you will need to use the mechanisms defined in the testbed `kvm-compose` tool.
For example, if you want to test the network connectivity in a guest, then you will need to use the `kvm-compose exec` tool.
Implicitly this will test that feature of the testbed as well.

For tests like network connectivity, these can be re-used so one has been implemented and placed in `shared_runtime_tests.py` in this folder.
You can take a look to see how it is implemented, and how the reporting is organised.
It is important that the tests uses the same format as these will then be automatically collected and presented in the final JSON report.


## Test Descriptions

### Base

This test case is the most basic yaml, spawn a few guests to test basic testbed features.
There will be no extra configuration that could create noise in whether the basic features work or not.
