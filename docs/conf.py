# -*- coding: utf-8 -*-
#
# sphinx-click documentation build configuration file

import os
import sys

sys.path.insert(0, os.path.abspath('examples'))

# -- General configuration ------------------------------------------------

# If your documentation needs a minimal Sphinx version, state it here.
#
needs_sphinx = '2.0'

# Add any Sphinx extension module names here, as strings. They can be
# extensions coming with Sphinx (named 'sphinx.ext.*') or your custom
# ones.
extensions = [
    'sphinx.ext.autosectionlabel',
]

autosectionlabel_prefix_document = True

# Add any paths that contain templates here, relative to this directory.
templates_path = []

# The suffix(es) of source filenames.
# You can specify multiple suffix as a list of string:
#
# source_suffix = ['.rst', '.md']
source_suffix = '.rst'

# The master toctree document.
master_doc = 'index'

# General information about the project.
project = 'testbed-os'
copyright = '2023, Bristol Cyber Security Group'
author = 'Bristol Cyber Security Group'

# The version info for the project you're documenting, acts as replacement for
# |version| and |release|, also used in various other places throughout the
# built documents.
#
# The short X.Y version.
version = '1.1'
# The full version, including alpha/beta/rc tags.
release = '1.1.0'

# List of patterns, relative to source directory, that match files and
# directories to ignore when looking for source files.
# This patterns also effect to html_static_path and html_extra_path
exclude_patterns = ['_build', 'Thumbs.db', '.DS_Store']

# The name of the Pygments (syntax highlighting) style to use.
pygments_style = 'sphinx'

# If true, `todo` and `todoList` produce output, else they produce nothing.
todo_include_todos = False


# -- Options for HTML output ----------------------------------------------

# The theme to use for HTML and HTML Help pages.  See the documentation for
# a list of builtin themes.
#
html_theme = 'sphinx_rtd_theme'


# -- Options for manual page output ---------------------------------------

# One entry per manual page. List of tuples
# (source start file, name, description, authors, manual section).
man_pages = [
    ("index", "testbed-os", "High level description of testbed OS", "BCSG", "1"),
    ("getting-started/index", "testbed-os-getting-started", "Guide to getting started with the testbed OS", "BCSG", "1"),
    ("installation/index", "testbed-os-installation", "Installation steps for the testbed OS", "BCSG", "1"),
    ("testbed-config/index", "testbed-os-config", "Documentation on the configuration of the testbed", "BCSG", "1"),
    ("kvm-compose/index", "kvm-compose", "Documentation on kvm-compose", "BCSG", "1"),
    ("orchestration/index", "kvm-orchestrator", "Documentation on the orchestration for the testbed", "BCSG", "1"),
    ("interfaces/index", "interfaces", "Documentation on interfaces for the testbed", "BCSG", "1"),
    ("guest-types/index", "guest-types", "Documentation on the types of guests available", "BCSG", "1"),
    ("testbedos-server/index", "testbedos-server", "Documentation on the testbedos server", "BCSG", "1"),
    ("networking/index", "kvm-compose-networking", "Documentation on the networking architecture for the testbed", "BCSG", "1"),
    ("resource-monitoring/index", "resource-monitoring", "Documentation on the resource monitoring for the testbed", "BCSG", "1"),
    ("test-harness/index", "test-harness", "Documentation on the test harness test suite for the testbed", "BCSG", "1"),
    ("examples/index", "examples", "Examples on how to use the testbed and create test cases", "BCSG", "1"),
]
