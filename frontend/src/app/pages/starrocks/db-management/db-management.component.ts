import { Component, OnInit } from '@angular/core';
import { Router } from '@angular/router';

@Component({
  selector: 'app-db-management',
  templateUrl: './db-management.component.html',
  styleUrls: ['./db-management.component.scss']
})
export class DbManagementComponent implements OnInit {

  constructor(private router: Router) { }

  ngOnInit(): void {
    // 默认导航到数据库管理页面
    this.router.navigate(['/pages/starrocks/db-management/databases']);
  }

}
