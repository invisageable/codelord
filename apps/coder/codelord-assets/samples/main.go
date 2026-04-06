package main

import (
	"fmt"
	"net/http"
	"encoding/json"
)

type Student struct {
	Id    int
	Name  string
	Class string
}

func main() {
	fmt.Println("Server running!!!")
	http.HandleFunc("/hello", helloHandler)
}

func helloHandler(rw http.ResponseWriter, req *http.Request) {
	students := []Student{
		{Id:1, Name:"Yanou", Class:"1A"},
		{Id:2, Name:"Tanaka", Class:"1B"},
	}

	data, _ := json.Marshal(students)
	rw.Write(data)
}
