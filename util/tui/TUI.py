#!/usr/bin/env python
# -*- coding: utf-8 -*-
import os
import sys
import time
from pathlib import Path
from pprint import pprint
import inquirer
from inquirer.themes import GreenPassion

import subprocess
import urllib.request

if __name__ == "__main__":

    project_dir = os.getcwd()
    print("Current working directory: " + str(project_dir))

    while(True):
        interface = [inquirer.List("application",
                           message="Which testbed application do you need?",
                           choices=["GUI", "TUI", "Exit"])]

        interface_choice = inquirer.prompt(interface, theme=GreenPassion())
             
        if interface_choice['application'] == "GUI":
            print("GUI is launching....")
            webUrl = urllib.request.urlopen("https://www.bristol.ac.uk/") 
            os.system('cls' if os.name == 'nt' else 'clear')
            break

        elif interface_choice['application'] == "Exit":
            os.system('cls' if os.name == 'nt' else 'clear')
            break
                
        elif interface_choice['application'] == "TUI":
            while(True):
                os.system('cls' if os.name == 'nt' else 'clear')
                kvm_command = [inquirer.List("testbed_command",
                            message="Which Testbed command do you need?",
                            choices=["Up", "Down", "Generate Artefacts", "Clear Artefacts", "Cloud Images", "Setup Config", "Deployment", "Snapshot", "Analysis Tools", "Help", "Exit TUI"])]
                _kvm_command = inquirer.prompt(kvm_command, theme=GreenPassion())
            

                if _kvm_command['testbed_command'] == "Exit TUI":
                    os.system('cls' if os.name == 'nt' else 'clear')
                    break
                
                elif _kvm_command['testbed_command'] == "Up":
                    os.system('cls' if os.name == 'nt' else 'clear')
                    subprocess.run(["kvm-compose", "up"])
                    enter = input(" ")
                    os.system('cls' if os.name == 'nt' else 'clear')
                            
                elif _kvm_command['testbed_command'] == "Down":
                    os.system('cls' if os.name == 'nt' else 'clear')
                    subprocess.run(["kvm-compose", "down"])
                    enter = input(" ")
                    os.system('cls' if os.name == 'nt' else 'clear')
                            
                elif _kvm_command['testbed_command'] == "Generate Artefacts":
                    os.system('cls' if os.name == 'nt' else 'clear')
                    subprocess.run(["kvm-compose", "generate-artefacts"])
                    enter = input(" ")
                    os.system('cls' if os.name == 'nt' else 'clear')

                elif _kvm_command['testbed_command'] == "Clear Artefacts":
                    os.system('cls' if os.name == 'nt' else 'clear')
                    subprocess.run(["kvm-compose", "clear-artefacts"])
                    enter = input(" ")
                    os.system('cls' if os.name == 'nt' else 'clear')

                elif _kvm_command['testbed_command'] == "Cloud Images":
                    os.system('cls' if os.name == 'nt' else 'clear')
                    subprocess.run(["kvm-compose", "cloud-images"])
                    enter = input(" ")
                    os.system('cls' if os.name == 'nt' else 'clear')

                elif _kvm_command['testbed_command'] == "Setup Config":
                    os.system('cls' if os.name == 'nt' else 'clear')
                    subprocess.run(["kvm-compose", "setup-config"])
                    enter = input(" ")
                    os.system('cls' if os.name == 'nt' else 'clear')

                elif _kvm_command['testbed_command'] == "Deployment":

                    while(True):
                        os.system('cls' if os.name == 'nt' else 'clear')
                        deployment_command = [inquirer.List("deploy_command",
                            message="Which Deployment command do you need?",
                            choices=["list", "info", "delete", "create", "help", "Exit Deployment commands"])]
                        _deploy_command = inquirer.prompt(deployment_command, theme=GreenPassion())

                        if _deploy_command['deploy_command'] == "Exit Deployment commands":
                            os.system('cls' if os.name == 'nt' else 'clear')
                            break

                        elif _deploy_command['deploy_command'] == "list":
                            os.system('cls' if os.name == 'nt' else 'clear')
                            dname = input("Enter name of deployment to list snapshots: ")
                            subprocess.run(["kvm-compose", "deployment", "list"] + dname)
                            enter = input(" ")
                            os.system('cls' if os.name == 'nt' else 'clear')

                        elif _deploy_command['deploy_command'] == "info":
                            os.system('cls' if os.name == 'nt' else 'clear')
                            dname = input("Enter name of deployment for info: ")
                            subprocess.run(["kvm-compose", "deployment", "info"] + dname)
                            enter = input(" ")
                            os.system('cls' if os.name == 'nt' else 'clear')

                        elif _deploy_command['deploy_command'] == "delete":
                            os.system('cls' if os.name == 'nt' else 'clear')
                            dname = input("Enter name of deployment to delete: ")
                            subprocess.run(["kvm-compose", "deployment", "delete"] + dname)
                            enter = input(" ")
                            os.system('cls' if os.name == 'nt' else 'clear')

                        elif _deploy_command['deploy_command'] == "create":
                            os.system('cls' if os.name == 'nt' else 'clear')
                            dname = input("Enter name of deployment to create: ")
                            subprocess.run(["kvm-compose", "deployment", "create"] + dname)
                            enter = input(" ")
                            os.system('cls' if os.name == 'nt' else 'clear')

                        elif _deploy_command['deploy_command'] == "help":
                            os.system('cls' if os.name == 'nt' else 'clear')
                            print("list        List all deployments \n"
                                "info        Get info on a deployment \n"
                                "delete      Destroy a deployment \n"
                                "create      Create a deployment"
                                )
                            enter = input(" ")
                            os.system('cls' if os.name == 'nt' else 'clear')


                elif _kvm_command['testbed_command'] == "Snapshot":

                    while(True):
                        os.system('cls' if os.name == 'nt' else 'clear')
                        snapshot_command = [inquirer.List("snapshots_command",
                            message="Which Snapshot command do you need?",
                            choices=["list", "info", "delete", "restore", "create", "help", "Exit Snapshot commands"])]
                        _snapshot_command = inquirer.prompt(snapshot_command, theme=GreenPassion())

                        if _snapshot_command['snapshots_command'] == "Exit Snapshot commands":
                            os.system('cls' if os.name == 'nt' else 'clear')
                            break
                        
                        elif _snapshot_command['snapshots_command'] == "list":
                            os.system('cls' if os.name == 'nt' else 'clear')
                            gname = input("Enter name of guest to list snapshots: ")
                            subprocess.run(["kvm-compose", "snapshot", "list"] + gname)
                            enter = input(" ")
                            os.system('cls' if os.name == 'nt' else 'clear')

                        elif _snapshot_command['snapshots_command'] == "info": # further commands need to be added
                            os.system('cls' if os.name == 'nt' else 'clear')
                            gname = input("Enter name of guest: ")
                            subprocess.run(["kvm-compose", "snapshot", "info"] + gname)
                            enter = input(" ")
                            os.system('cls' if os.name == 'nt' else 'clear')
                        
                        elif _snapshot_command['snapshots_command'] == "delete":
                            os.system('cls' if os.name == 'nt' else 'clear')
                            gname = input("Enter name of guest: ")
                            # sname = input("Enter name of snapshot: ")
                            enter = input(" ")
                            os.system('cls' if os.name == 'nt' else 'clear')
                        
                        elif _snapshot_command['snapshots_command'] == "restore":
                            os.system('cls' if os.name == 'nt' else 'clear')
                            gname = input("Enter name of guest: ")
                            sname = input("Enter name of snapshot: ")
                            enter = input(" ")
                            os.system('cls' if os.name == 'nt' else 'clear')

                        elif _snapshot_command['snapshots_command'] == "create":
                            os.system('cls' if os.name == 'nt' else 'clear')
                            gname = input("Enter name of guest: ")
                            enter = input(" ")
                            os.system('cls' if os.name == 'nt' else 'clear')

    
                elif _kvm_command['testbed_command'] == "Analysis Tools":

                    while(True):
                        os.system('cls' if os.name == 'nt' else 'clear')
                        analysis_command = [inquirer.List("analysis_tools_command",
                            message="Which Analysis Tool do you need?",
                            choices=["tcpdump", "topology", "resource monitor", "help", "Exit Analysis Tools"])]
                        _analysis_command = inquirer.prompt(analysis_command, theme=GreenPassion())

                        if _analysis_command['analysis_tools_command'] == "Exit Analysis Tools":
                            os.system('cls' if os.name == 'nt' else 'clear')
                            break

                        elif _analysis_command['analysis_tools_command'] == "tcpdump":
                                while(True):
                                    os.system('cls' if os.name == 'nt' else 'clear')
                                    tcpdump_command = [inquirer.List("tcp_dump_command",
                                        message="Which tcpdump option do you need?",
                                        choices=["<your_commands>", "help", "Exit tcpdump"])]
                                    _tcpdump_command = inquirer.prompt(tcpdump_command, theme=GreenPassion())

                                    if _tcpdump_command['tcp_dump_command'] == "Exit tcpdump":
                                        os.system('cls' if os.name == 'nt' else 'clear')
                                        break

                                    elif  _tcpdump_command['tcp_dump_command'] == "help":
                                        os.system('cls' if os.name == 'nt' else 'clear')
                                        print("[ -C file_size ] [ -E algo:secret ] [ -F file ] [ -G seconds ] \n"
                                            "[ -i interface ] [ --immediate-mode ] [ -j tstamptype ] \n"
                                            "[ -M secret ] [ --number ] [ --print ] [ -Q in|out|inout ] \n"
                                            "[ -r file ] [ -s snaplen ] [ -T type ] [ --version ] \n"
                                            "[ -V file ] [ -w file ] [ -W filecount ] [ -y datalinktype ] \n"
                                            "[ --time-stamp-precision precision ] [ --micro ] [ --nano ] \n"
                                            "[ -z postrotate-command ] [ -Z user ] [ expression ]")
                                        enter = input(" ")
                                        os.system('cls' if os.name == 'nt' else 'clear')

                                    elif _tcpdump_command['tcp_dump_command'] == "<your_commands>":
                                        os.system('cls' if os.name == 'nt' else 'clear')
                                        list_of_arguments = input("Enter the arguments: ")
                                        _list_of_arguments = list_of_arguments.split()
                                        # subprocess.run(["kvm-compose", "analysis-tools", "tcp-dump"] + "--" + _list_of_arguments)
                                        enter = input(" ")
                                        os.system('cls' if os.name == 'nt' else 'clear')

                        elif _analysis_command['analysis_tools_command'] == "topology":
                            os.system('cls' if os.name == 'nt' else 'clear')
                            subprocess.run(["kvm-compose", "analysis-tools", "network-topology"])
                            enter = input(" ")
                            os.system('cls' if os.name == 'nt' else 'clear')

                        # elif _analysis_command['analysis_tools_command'] == "resource monitor":
                        #     os.system('cls' if os.name == 'nt' else 'clear')
                        #     # subprocess.run(["kvm-compose", "analysis-tools", "resource-monitor"])
                        #     enter = input(" ")
                        #     os.system('cls' if os.name == 'nt' else 'clear')

                        elif _analysis_command['analysis_tools_command'] == "help":
                            os.system('cls' if os.name == 'nt' else 'clear')
                            print("tcpdump                  Capture and analyse network traffic \n"
                                "topology                 Visualize network topology \n"
                                "resource monitor         Monitor the resources status  \n"
                                "Exit Analysis Tools      Exit from analysis tools option \n")
                            enter = input(" ")
                            os.system('cls' if os.name == 'nt' else 'clear')
                            
            
                elif _kvm_command['testbed_command'] == "Help":
                    os.system('cls' if os.name == 'nt' else 'clear')
                    print("Up                   Deploy the test case \n"
                            "Down                 Undeploy the test case \n"
                            "Generate Artefacts   Create all artefacts for virtual devices in the current configuration \n"
                            "Clear Artefacts      Destroy all artefacts for the current configuration \n"
                            "Cloud Images         List supported cloud images \n"
                            "Setup Config         Setup kvm compose config \n"
                            "Deployment           Control deployments on the testbed server \n"
                            "Snapshot             Snapshot guests \n"
                            "Analysis Tools       Analysis Tools \n"
                            )
                    enter = input("Press enter to return to the previous menu")
                    os.system('cls' if os.name == 'nt' else 'clear')
                
                    
        
