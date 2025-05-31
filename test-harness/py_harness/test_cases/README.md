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

    def test_case(self) -> bool:  # <------------ 2.
        
        # Here you implement tests inside the guests defined in the yaml
        
        return True  # Return whether the test case worked or not

registered_test_cases.append(BaseTestCase)  # <-- 3.
```


## Test Descriptions

### Base

This test case is the most basic yaml, spawn a few guests to test basic testbed features.
There will be no extra configuration that could create noise in whether the basic features work or not.
