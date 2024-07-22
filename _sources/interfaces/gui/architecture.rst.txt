GUI Architecture
================

The GUI is a web application that is hosted by the testbed server.
It uses serverside HTML rendering using teradocs_, and a combination of bootstrap and jQuery.
This has been kept to a minimum to keep the site simple, functional, and easy to maintain.

The dynamic portions of the GUI utilise the testbed's api to get information about deployments.
It also supports running commands such as orchestration and analysis tools, directly from the GUI.

Command Running
---------------

Command running is supported via a websocket connection from the browser to the testbed server api.
The GUI also provides a pseudo terminal to emulate the CLI behaviour to display the logs of commands as they appear.
As commands are executed on the server, the server pushes logs to the GUI back over the websocket.
Note that the logging is not one to one with what you would see if using the command line, it is mostly a general indication of what is happening so that there is some activity to be seen by the user.
For more detailed logging, especially if there was a problem, you should defer to the server logs for a full picture.

To keep things simple, the command running from the GUI is essentially a wrapper around the CLI code.
This means that the same code the CLI would use to trigger a command to the server is used in a special endpoint just for the GUI.
In essence, this special endpoint on the server will open up another websocket to the server for the original command running code path.
This was the path chosen in favour of partially re-implementing the command running in javascript, using endpoints to control the filesystem rather than the browser.
Another possibility would have been to run the CLI as a subprocess, but we are moving away from subprocess use.

Authentication
--------------

There are currently no users and login for the GUI.
While the testbed is un-authenticated, we have not added this but in a future release when the endpoints are secured we will also add users to the GUI.

.. _teradocs: https://keats.github.io/tera/docs/
